// In-browser matching engine simulacrum. Speaks the gateway's exact wire format
// (ServerMessage) so the UI cannot tell it apart from the Rust engine. Used when
// no gateway is reachable (e.g. the static Netlify deployment).
//
// Model: an Ornstein-Uhlenbeck fair price, Poisson limit/market order flow around
// the touch, price-time priority matching, and level decay far from the touch.

import type { Level, NewOrderBody, ServerMessage, Side } from '../types'

// All prices are integer ticks (1 tick = 0.01 quote units), matching the gateway wire.
const BOOK_DEPTH = 18
const STEP_MS = 75
const SNAPSHOT_EVERY = 2
const BASE_TICKS = 18740 // 187.40
const MEAN_REVERSION = 0.02
const VOLATILITY = 2.4
const LEVEL_DECAY_P = 0.04
const MAX_RESTING = 400
const FAR_FROM_TOUCH = 12

interface RestingOrder {
  id: number
  side: Side
  price: number // integer ticks
  quantity: number
  user: boolean
}

function randQty(): number {
  return Math.max(1, Math.round(Math.exp(Math.random() * 3.4)))
}

export class LocalBourse {
  private bids: RestingOrder[] = [] // sorted desc by price, FIFO within level
  private asks: RestingOrder[] = [] // sorted asc by price, FIFO within level
  private fair = BASE_TICKS
  private seq = 0
  private nextId = 1
  private timer: ReturnType<typeof setInterval> | undefined
  private step = 0
  private intensity = 1
  private emit: (m: ServerMessage) => void = () => {}

  start(onMessage: (m: ServerMessage) => void): void {
    this.emit = onMessage
    this.seed()
    this.snapshot()
    this.timer = setInterval(() => this.tick(), STEP_MS)
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer)
    this.timer = undefined
  }

  /** Maps the sim-rate control (orders/sec) onto flow intensity. */
  setIntensity(rate: number): void {
    this.intensity = Math.min(6, Math.max(0.2, rate / 40))
  }

  submit(body: NewOrderBody): { id: number; accepted: boolean } {
    const id = this.nextId++
    const quantity = Math.floor(body.quantity)
    if (quantity <= 0) {
      this.emit({ type: 'order_rejected', id, reason: 'quantity must be positive' })
      return { id, accepted: false }
    }
    if (body.order_type === 'market') {
      this.emit({ type: 'order_accepted', id })
      this.executeMarket(body.side, quantity, id)
      this.snapshot()
      return { id, accepted: true }
    }
    if (body.price === undefined || body.price <= 0) {
      this.emit({ type: 'order_rejected', id, reason: 'limit order requires a price' })
      return { id, accepted: false }
    }
    this.emit({ type: 'order_accepted', id })
    this.placeLimit(body.side, Math.round(body.price), quantity, id, true)
    this.snapshot()
    return { id, accepted: true }
  }

  cancel(id: number): boolean {
    for (const book of [this.bids, this.asks]) {
      const i = book.findIndex((o) => o.id === id)
      if (i >= 0) {
        const [o] = book.splice(i, 1)
        this.emit({ type: 'order_canceled', id, remaining: o.quantity })
        this.snapshot()
        return true
      }
    }
    return false
  }

  // ── internals ──────────────────────────────────────────────────────────

  private seed(): void {
    for (let d = 1; d <= BOOK_DEPTH + 6; d++) {
      for (let n = 0; n < 2; n++) {
        this.rest('buy', this.fair - d, randQty())
        this.rest('sell', this.fair + d, randQty())
      }
    }
  }

  private tick(): void {
    this.step++
    const noise = (Math.random() - 0.5) * 2
    this.fair += MEAN_REVERSION * (BASE_TICKS - this.fair) + noise * VOLATILITY

    const flow = Math.ceil(this.intensity * (1 + Math.random() * 2))
    for (let i = 0; i < flow; i++) {
      if (Math.random() < 0.72) {
        const side: Side = Math.random() < 0.5 ? 'buy' : 'sell'
        const offset = 1 + Math.floor(Math.random() * 10 * Math.random())
        const price =
          side === 'buy' ? Math.round(this.fair) - offset : Math.round(this.fair) + offset
        this.placeLimit(side, price, randQty(), this.nextId++, false)
      } else {
        const side: Side = Math.random() < 0.5 ? 'buy' : 'sell'
        this.executeMarket(side, randQty(), this.nextId++)
      }
    }

    this.decay()
    if (this.step % SNAPSHOT_EVERY === 0) this.snapshot()
  }

  private rest(side: Side, price: number, quantity: number, id?: number, user = false): void {
    const book = side === 'buy' ? this.bids : this.asks
    book.push({ id: id ?? this.nextId++, side, price, quantity, user })
    book.sort((a, b) => (side === 'buy' ? b.price - a.price : a.price - b.price))
  }

  private placeLimit(side: Side, price: number, quantity: number, id: number, user: boolean): void {
    let remaining = quantity
    const opposite = side === 'buy' ? this.asks : this.bids
    const crosses = (top: RestingOrder) =>
      side === 'buy' ? price >= top.price : price <= top.price

    while (remaining > 0 && opposite.length > 0 && crosses(opposite[0])) {
      remaining = this.fill(opposite, remaining, side, id)
    }
    if (remaining > 0) this.rest(side, price, remaining, id, user)
  }

  private executeMarket(side: Side, quantity: number, takerId: number): void {
    let remaining = quantity
    const opposite = side === 'buy' ? this.asks : this.bids
    while (remaining > 0 && opposite.length > 0) {
      remaining = this.fill(opposite, remaining, side, takerId)
    }
  }

  private fill(opposite: RestingOrder[], remaining: number, takerSide: Side, takerId: number): number {
    const maker = opposite[0]
    const traded = Math.min(remaining, maker.quantity)
    maker.quantity -= traded
    if (maker.quantity === 0) opposite.shift()
    this.emit({
      type: 'trade',
      seq: ++this.seq,
      price: maker.price,
      quantity: traded,
      taker_side: takerSide,
      taker_order: takerId,
      maker_order: maker.id,
    })
    return remaining - traded
  }

  /** Thin out stale liquidity far from the touch; keeps the book breathing. */
  private decay(): void {
    for (const book of [this.bids, this.asks]) {
      for (let i = book.length - 1; i >= 0; i--) {
        if (book[i].user) continue
        const far = Math.abs(book[i].price - this.fair) > FAR_FROM_TOUCH
        if (Math.random() < LEVEL_DECAY_P * (far ? 2 : 0.5)) book.splice(i, 1)
      }
      if (book.length > MAX_RESTING) book.length = MAX_RESTING
    }
  }

  private levels(book: RestingOrder[]): Level[] {
    const byPrice = new Map<number, number>()
    for (const o of book) {
      byPrice.set(o.price, (byPrice.get(o.price) ?? 0) + o.quantity)
    }
    return [...byPrice.entries()]
      .slice(0, BOOK_DEPTH)
      .map(([price, quantity]) => ({ price, quantity }))
  }

  private snapshot(): void {
    const bids = this.levels(this.bids)
    const asks = this.levels(this.asks)
    const best_bid = bids[0]?.price ?? null
    const best_ask = asks[0]?.price ?? null
    this.emit({
      type: 'book',
      bids,
      asks,
      seq: ++this.seq,
      best_bid,
      best_ask,
      spread: best_bid !== null && best_ask !== null ? best_ask - best_bid : null,
    })
  }
}
