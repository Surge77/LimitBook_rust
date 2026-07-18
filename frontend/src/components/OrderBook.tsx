// DOM ladder in the TradingView tradition: asks stack down to a center spread
// row, bids below; per-row depth bars ease width; changed rows flash their side.

import { memo, useEffect, useRef, useState } from 'react'
import type { BookMsg, Level } from '../types'
import { fmtPrice, fmtQty } from '../lib/format'

const DEPTH = 12

function maxQty(levels: Level[]): number {
  return levels.reduce((m, l) => Math.max(m, l.quantity), 1)
}

const Row = memo(function Row({
  level,
  side,
  max,
  cum,
}: {
  level: Level
  side: 'up' | 'down'
  max: number
  cum: number
}) {
  const [flash, setFlash] = useState(false)
  const prevQty = useRef(level.quantity)
  useEffect(() => {
    if (prevQty.current !== level.quantity) {
      prevQty.current = level.quantity
      setFlash(true)
      const t = setTimeout(() => setFlash(false), 500)
      return () => clearTimeout(t)
    }
  }, [level.quantity])

  const isUp = side === 'up'
  return (
    <div
      className={`relative grid grid-cols-3 items-center px-3 py-[2.5px] hover:bg-raised ${
        flash ? (isUp ? 'flash-up' : 'flash-down') : ''
      }`}
    >
      <div
        className={`depth-bar absolute inset-y-0 right-0 ${isUp ? 'bg-up/12' : 'bg-down/12'}`}
        style={{ width: `${Math.min(100, (level.quantity / max) * 100)}%` }}
      />
      <span className={`mono relative text-[12.5px] ${isUp ? 'text-up-text' : 'text-down-text'}`}>
        {fmtPrice(level.price)}
      </span>
      <span className="mono relative text-right text-[12.5px] text-text">
        {fmtQty(level.quantity)}
      </span>
      <span className="mono relative text-right text-[12.5px] text-text-3">{fmtQty(cum)}</span>
    </div>
  )
})

export function OrderBook({ book }: { book: BookMsg | null }) {
  const bids = book?.bids.slice(0, DEPTH) ?? []
  const asks = book?.asks.slice(0, DEPTH) ?? []
  const max = maxQty([...bids, ...asks])

  let cum = 0
  const bidRows = bids.map((l) => ({ level: l, cum: (cum += l.quantity) }))
  cum = 0
  const askRows = asks.map((l) => ({ level: l, cum: (cum += l.quantity) }))
  const asksDisplay = [...askRows].reverse()

  const mid =
    book?.best_bid != null && book?.best_ask != null
      ? fmtPrice(Math.round((book.best_bid + book.best_ask) / 2))
      : '—'

  return (
    <div className="tpanel flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between border-b border-line px-3 py-2">
        <span className="tlabel">Order book</span>
        <span className="mono text-[10px] text-text-3">0.01</span>
      </div>
      <div className="grid grid-cols-3 border-b border-line-soft px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-text-3">
        <span>Price</span>
        <span className="text-right">Size</span>
        <span className="text-right">Sum</span>
      </div>

      <div className="flex min-h-0 flex-1 flex-col justify-end overflow-hidden">
        {asksDisplay.map((r) => (
          <Row key={`a-${r.level.price}`} level={r.level} side="down" max={max} cum={r.cum} />
        ))}
      </div>

      <div className="flex items-center justify-between border-y border-line bg-raised px-3 py-1.5">
        <span className="mono text-[15px] font-semibold">{mid}</span>
        <span className="mono text-[10px] text-text-3">
          spread {fmtPrice(book?.spread)}
        </span>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {bidRows.map((r) => (
          <Row key={`b-${r.level.price}`} level={r.level} side="up" max={max} cum={r.cum} />
        ))}
      </div>
    </div>
  )
}
