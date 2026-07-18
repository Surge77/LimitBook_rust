// Top bar in the tradition of exchange workstations: instrument + animated last
// price on the left (Binance-style stat strip), engine health on the right, and
// a Bloomberg-amber command strip underneath.

import { useEffect, useMemo, useRef, useState } from 'react'
import type { FeedState } from '../hooks/useEngineFeed'
import { fmtPrice } from '../lib/format'
import { Odometer } from './Odometer'

function Stat({ label, value, tone }: { label: string; value: string; tone?: 'up' | 'down' }) {
  const color =
    tone === 'up' ? 'text-up-text' : tone === 'down' ? 'text-down-text' : 'text-text'
  return (
    <div className="flex flex-col gap-0.5 border-l border-line-soft pl-4">
      <span className="text-[10px] font-medium uppercase tracking-wide text-text-3">{label}</span>
      <span className={`mono text-[13px] font-medium ${color}`}>{value}</span>
    </div>
  )
}

export function Masthead({ feed }: { feed: FeedState }) {
  const last = feed.trades[0]
  const lastPrice = last ? fmtPrice(last.price) : '—'
  const direction = useMemo(() => {
    const [a, b] = feed.trades
    if (!a || !b || a.price === b.price) return null
    return a.price > b.price ? 'up' : 'down'
  }, [feed.trades])

  const [tickCls, setTickCls] = useState('')
  const lastSeq = useRef<number | null>(null)
  useEffect(() => {
    if (last && last.seq !== lastSeq.current) {
      lastSeq.current = last.seq
      setTickCls('')
      requestAnimationFrame(() =>
        setTickCls(direction === 'down' ? 'tick-down' : 'tick-up'),
      )
    }
  }, [last, direction])

  return (
    <header className="tpanel border-x-0 border-t-0">
      <div className="flex items-center gap-5 px-4 py-2.5">
        <div className="flex items-center gap-3 pr-1">
          <div className="flex h-7 w-7 items-center justify-center border border-amber/60 bg-amber/10">
            <span className="mono text-[11px] font-bold text-amber">LB</span>
          </div>
          <div className="leading-tight">
            <div className="flex items-center gap-2">
              <span className="text-[14px] font-semibold tracking-tight">LMB / USD</span>
              <span className="border border-line bg-raised px-1.5 py-px text-[9px] font-semibold uppercase tracking-wider text-text-2">
                Spot
              </span>
            </div>
            <span className="text-[10px] text-text-3">LimitBook · Rust matching engine</span>
          </div>
        </div>

        <div
          className={`mono text-[26px] font-semibold leading-none tracking-tight ${
            direction === 'down' ? 'text-down-text' : 'text-up-text'
          } ${tickCls}`}
        >
          <Odometer value={lastPrice} />
          <span className="ml-1.5 align-middle text-[12px]">
            {direction === 'down' ? '▼' : '▲'}
          </span>
        </div>

        <div className="flex flex-1 items-center gap-4 overflow-x-auto pl-2">
          <Stat label="Best bid" value={fmtPrice(feed.book?.best_bid)} tone="up" />
          <Stat label="Best ask" value={fmtPrice(feed.book?.best_ask)} tone="down" />
          <Stat label="Spread" value={fmtPrice(feed.book?.spread)} />
          <Stat label="Trades/s" value={String(feed.tradesPerSec)} />
          <Stat label="Orders/s" value={String(feed.ordersPerSec)} />
          <Stat label="Seq" value={feed.book ? feed.book.seq.toLocaleString() : '—'} />
        </div>

        <ConnectionBadge mode={feed.mode} connected={feed.connected} />
      </div>

      <div className="flex items-center gap-2 border-t border-line-soft bg-[#0d1016] px-4 py-1.5">
        <span className="mono text-[11px] font-semibold text-amber">&gt;</span>
        <span className="caret mono text-[11px] text-text-3">
          LMB &lt;GO&gt; — price-time priority · zero-GC hot path · {'{'}limit, market, IOC, FOK,
          post-only, stop{'}'}
        </span>
      </div>
    </header>
  )
}

function ConnectionBadge({ mode, connected }: { mode: string; connected: boolean }) {
  const live = mode === 'gateway'
  return (
    <div className="flex shrink-0 items-center gap-2 border border-line bg-raised px-2.5 py-1.5">
      <span
        className={`pulse-dot inline-block h-1.5 w-1.5 rounded-full ${
          connected ? (live ? 'bg-up-text' : 'bg-amber') : 'bg-down-text'
        }`}
      />
      <div className="leading-tight">
        <div className="text-[10px] font-semibold uppercase tracking-wider">
          {live ? 'Live engine' : 'Sim feed'}
        </div>
        <div className="mono text-[9px] text-text-3">
          {connected ? (live ? 'ws://gateway' : 'in-browser matcher') : 'reconnecting…'}
        </div>
      </div>
    </div>
  )
}
