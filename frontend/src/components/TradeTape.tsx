// Recent trades feed (exchange convention: price colored by taker side, newest
// on top, subtle slide-in) plus the thin scrolling ticker strip for the footer.

import type { TradeMsg } from '../types'
import { fmtPrice, fmtQty } from '../lib/format'

export function TradeTape({ trades }: { trades: TradeMsg[] }) {
  return (
    <div className="tpanel flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between border-b border-line px-3 py-2">
        <span className="tlabel">Recent trades</span>
        <span className="mono text-[10px] text-text-3">{trades.length}</span>
      </div>
      <div className="grid grid-cols-3 border-b border-line-soft px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-text-3">
        <span>Price</span>
        <span className="text-right">Size</span>
        <span className="text-right">Seq</span>
      </div>
      <div className="min-h-0 flex-1 overflow-hidden">
        {trades.length === 0 && (
          <div className="px-3 py-4 text-center text-[11px] text-text-3">no trades yet</div>
        )}
        {trades.map((t) => (
          <div
            key={t.seq}
            className="tape-in grid grid-cols-3 items-center px-3 py-[2.5px] hover:bg-raised"
          >
            <span
              className={`mono text-[12.5px] ${
                t.taker_side === 'buy' ? 'text-up-text' : 'text-down-text'
              }`}
            >
              {fmtPrice(t.price)}
            </span>
            <span className="mono text-right text-[12.5px] text-text">{fmtQty(t.quantity)}</span>
            <span className="mono text-right text-[11px] text-text-3">{t.seq}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

/** Thin scrolling strip of prints for the status footer. */
export function TickerRibbon({ trades }: { trades: TradeMsg[] }) {
  const items = trades.slice(0, 24)
  if (items.length === 0) return null
  const strip = (
    <>
      {items.map((t) => (
        <span key={t.seq} className="mono mx-5 inline-flex items-baseline gap-1.5 text-[11px]">
          <span className="text-text-3">LMB</span>
          <span className={t.taker_side === 'buy' ? 'text-up-text' : 'text-down-text'}>
            {fmtPrice(t.price)}
          </span>
          <span className="text-text-3">×{fmtQty(t.quantity)}</span>
        </span>
      ))}
    </>
  )
  return (
    <div className="overflow-hidden whitespace-nowrap" aria-hidden>
      <div className="marquee inline-block py-1">
        {strip}
        {strip}
      </div>
    </div>
  )
}
