import type { BookMsg, Level } from '../types'
import { fmtPrice, fmtQty } from '../lib/format'

interface OrderBookProps {
  book: BookMsg | null
}

const DEPTH = 12

function maxQty(levels: Level[]): number {
  return levels.reduce((m, l) => Math.max(m, l.quantity), 1)
}

function Row({ level, side, max }: { level: Level; side: 'bid' | 'ask'; max: number }) {
  const width = `${Math.min(100, (level.quantity / max) * 100)}%`
  const barColor = side === 'bid' ? 'bg-emerald-500/15' : 'bg-rose-500/15'
  const priceColor = side === 'bid' ? 'text-emerald-400' : 'text-rose-400'
  return (
    <div className="relative grid grid-cols-2 px-3 py-0.5 text-sm tabular-nums">
      <div className={`absolute inset-y-0 right-0 ${barColor}`} style={{ width }} />
      <span className={`relative z-10 ${priceColor}`}>{fmtPrice(level.price)}</span>
      <span className="relative z-10 text-right text-zinc-300">{fmtQty(level.quantity)}</span>
    </div>
  )
}

export function OrderBook({ book }: OrderBookProps) {
  const bids = book?.bids.slice(0, DEPTH) ?? []
  const asks = book?.asks.slice(0, DEPTH) ?? []
  const max = maxQty([...bids, ...asks])
  const asksDisplay = [...asks].reverse() // best ask nearest the spread

  return (
    <div className="flex h-full flex-col rounded-lg border border-zinc-800 bg-zinc-950">
      <div className="grid grid-cols-2 border-b border-zinc-800 px-3 py-1.5 text-xs uppercase tracking-wide text-zinc-500">
        <span>Price</span>
        <span className="text-right">Size</span>
      </div>
      <div className="flex flex-1 flex-col justify-end">
        {asksDisplay.map((l) => (
          <Row key={`a-${l.price}`} level={l} side="ask" max={max} />
        ))}
      </div>
      <div className="border-y border-zinc-800 px-3 py-1.5 text-center text-sm tabular-nums text-zinc-400">
        {book?.best_bid != null && book?.best_ask != null ? (
          <span>
            <span className="text-emerald-400">{fmtPrice(book.best_bid)}</span>
            <span className="mx-2 text-zinc-600">/</span>
            <span className="text-rose-400">{fmtPrice(book.best_ask)}</span>
            <span className="ml-3 text-zinc-500">spread {fmtPrice(book.spread)}</span>
          </span>
        ) : (
          <span className="text-zinc-600">no market</span>
        )}
      </div>
      <div className="flex flex-1 flex-col">
        {bids.map((l) => (
          <Row key={`b-${l.price}`} level={l} side="bid" max={max} />
        ))}
      </div>
    </div>
  )
}
