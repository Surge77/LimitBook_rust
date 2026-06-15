import type { FeedState } from '../hooks/useEngineFeed'
import { fmtInt, fmtPrice } from '../lib/format'

function Stat({ label, value, accent }: { label: string; value: string; accent?: string }) {
  return (
    <div className="rounded border border-zinc-800 bg-zinc-900 px-3 py-2">
      <div className="text-[10px] uppercase tracking-wide text-zinc-500">{label}</div>
      <div className={`text-lg tabular-nums ${accent ?? 'text-zinc-100'}`}>{value}</div>
    </div>
  )
}

export function MetricsPanel({ feed }: { feed: FeedState }) {
  const { book, tradesPerSec, ordersPerSec, connected } = feed
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3">
      <div className="flex items-center justify-between text-xs uppercase tracking-wide text-zinc-500">
        <span>Metrics</span>
        <span className={connected ? 'text-emerald-400' : 'text-rose-400'}>
          {connected ? '● live' : '○ offline'}
        </span>
      </div>
      <div className="grid grid-cols-2 gap-2">
        <Stat label="Best Bid" value={fmtPrice(book?.best_bid)} accent="text-emerald-400" />
        <Stat label="Best Ask" value={fmtPrice(book?.best_ask)} accent="text-rose-400" />
        <Stat label="Spread" value={fmtPrice(book?.spread)} />
        <Stat label="Trades/s" value={fmtInt(tradesPerSec)} accent="text-sky-400" />
        <Stat label="Orders/s" value={fmtInt(ordersPerSec)} accent="text-amber-400" />
        <Stat label="Seq" value={book ? fmtInt(book.seq) : '—'} />
      </div>
    </div>
  )
}
