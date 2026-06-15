import type { TradeMsg } from '../types'
import { fmtPrice, fmtQty } from '../lib/format'

interface TradeTapeProps {
  trades: TradeMsg[]
}

export function TradeTape({ trades }: TradeTapeProps) {
  return (
    <div className="flex h-full flex-col rounded-lg border border-zinc-800 bg-zinc-950">
      <div className="grid grid-cols-3 border-b border-zinc-800 px-3 py-1.5 text-xs uppercase tracking-wide text-zinc-500">
        <span>Price</span>
        <span className="text-right">Size</span>
        <span className="text-right">Seq</span>
      </div>
      <div className="flex-1 overflow-y-auto">
        {trades.length === 0 && (
          <div className="px-3 py-2 text-sm text-zinc-600">no trades yet</div>
        )}
        {trades.map((t) => {
          const color = t.taker_side === 'buy' ? 'text-emerald-400' : 'text-rose-400'
          return (
            <div
              key={t.seq}
              className="grid grid-cols-3 px-3 py-0.5 text-sm tabular-nums"
            >
              <span className={color}>{fmtPrice(t.price)}</span>
              <span className="text-right text-zinc-300">{fmtQty(t.quantity)}</span>
              <span className="text-right text-zinc-600">{t.seq}</span>
            </div>
          )
        })}
      </div>
    </div>
  )
}
