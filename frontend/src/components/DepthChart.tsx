// Market depth in the TradingView idiom: gradient-filled cumulative step areas,
// green west of mid, red east, crosshair + tooltip on hover. Hand-rolled SVG.

import { useMemo, useRef, useState } from 'react'
import type { BookMsg } from '../types'
import { fmtPrice, fmtQty } from '../lib/format'

const W = 640
const H = 220
const PAD = { top: 12, right: 8, bottom: 22, left: 8 }

interface CumPoint {
  price: number
  cum: number
}

function cumulative(levels: { price: number; quantity: number }[]): CumPoint[] {
  let cum = 0
  return levels.map((l) => ({ price: l.price, cum: (cum += l.quantity) }))
}

function stepPath(
  pts: CumPoint[],
  x: (p: number) => number,
  y: (c: number) => number,
  baseline: number,
): string {
  if (pts.length === 0) return ''
  let d = `M ${x(pts[0].price)} ${baseline} L ${x(pts[0].price)} ${y(pts[0].cum)}`
  for (let i = 1; i < pts.length; i++) {
    d += ` L ${x(pts[i].price)} ${y(pts[i - 1].cum)} L ${x(pts[i].price)} ${y(pts[i].cum)}`
  }
  d += ` L ${x(pts[pts.length - 1].price)} ${baseline} Z`
  return d
}

export function DepthChart({ book }: { book: BookMsg | null }) {
  const svgRef = useRef<SVGSVGElement>(null)
  const [hover, setHover] = useState<{ px: number; price: number } | null>(null)

  const model = useMemo(() => {
    if (!book || book.bids.length === 0 || book.asks.length === 0) return null
    const bids = cumulative(book.bids)
    const asks = cumulative(book.asks)
    const lo = bids[bids.length - 1].price
    const hi = asks[asks.length - 1].price
    const maxCum = Math.max(bids[bids.length - 1].cum, asks[asks.length - 1].cum)
    const x = (p: number) => PAD.left + ((p - lo) / (hi - lo || 1)) * (W - PAD.left - PAD.right)
    const y = (c: number) => H - PAD.bottom - (c / maxCum) * (H - PAD.top - PAD.bottom)
    return { bids, asks, lo, hi, x, y }
  }, [book])

  if (!model || !book) {
    return (
      <div className="tpanel flex h-full items-center justify-center">
        <span className="tlabel">awaiting depth</span>
      </div>
    )
  }

  const { bids, asks, lo, hi, x, y } = model
  const baseline = H - PAD.bottom

  const onMove = (e: React.MouseEvent<SVGSVGElement>) => {
    const rect = svgRef.current?.getBoundingClientRect()
    if (!rect) return
    const px = ((e.clientX - rect.left) / rect.width) * W
    const price = lo + ((px - PAD.left) / (W - PAD.left - PAD.right)) * (hi - lo)
    setHover(price >= lo && price <= hi ? { px, price } : null)
  }

  const hoverInfo = hover
    ? hover.price <= (book.best_bid ?? -Infinity)
      ? { side: 'bid' as const, pts: bids.filter((p) => p.price >= hover.price) }
      : hover.price >= (book.best_ask ?? Infinity)
        ? { side: 'ask' as const, pts: asks.filter((p) => p.price <= hover.price) }
        : null
    : null
  const hoverCum = hoverInfo?.pts.length ? hoverInfo.pts[hoverInfo.pts.length - 1].cum : null

  return (
    <div className="tpanel relative flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between border-b border-line px-3 py-2">
        <span className="tlabel">Depth</span>
        <span className="text-[10px] text-text-3">
          <span className="text-up-text">● bids</span>
          <span className="mx-1.5">·</span>
          <span className="text-down-text">● asks</span>
        </span>
      </div>
      <svg
        ref={svgRef}
        viewBox={`0 0 ${W} ${H}`}
        className="chart-surface min-h-0 w-full flex-1"
        preserveAspectRatio="none"
        onMouseMove={onMove}
        onMouseLeave={() => setHover(null)}
        role="img"
        aria-label="Cumulative order book depth by price"
      >
        <defs>
          <linearGradient id="g-bid" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="rgba(10,165,116,0.35)" />
            <stop offset="100%" stopColor="rgba(10,165,116,0.04)" />
          </linearGradient>
          <linearGradient id="g-ask" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="rgba(229,72,77,0.35)" />
            <stop offset="100%" stopColor="rgba(229,72,77,0.04)" />
          </linearGradient>
        </defs>

        {[0.25, 0.5, 0.75].map((f) => (
          <line key={f} x1={PAD.left} x2={W - PAD.right}
            y1={PAD.top + f * (H - PAD.top - PAD.bottom)}
            y2={PAD.top + f * (H - PAD.top - PAD.bottom)}
            stroke="#1a212b" strokeWidth="1" />
        ))}

        <path d={stepPath(bids, x, y, baseline)} fill="url(#g-bid)"
          stroke="#0aa574" strokeWidth="1.5" />
        <path d={stepPath(asks, x, y, baseline)} fill="url(#g-ask)"
          stroke="#e5484d" strokeWidth="1.5" />

        <line x1={PAD.left} x2={W - PAD.right} y1={baseline} y2={baseline}
          stroke="#232a35" strokeWidth="1" />

        {book.best_bid != null && book.best_ask != null && (
          <line x1={x((book.best_bid + book.best_ask) / 2)}
            x2={x((book.best_bid + book.best_ask) / 2)}
            y1={PAD.top} y2={baseline} stroke="#ffb224" strokeWidth="0.8"
            strokeDasharray="2 3" opacity="0.7" />
        )}

        {hover && (
          <line x1={hover.px} x2={hover.px} y1={PAD.top} y2={baseline}
            stroke="#8b96a5" strokeWidth="0.8" strokeDasharray="3 3" />
        )}

        <text x={PAD.left} y={H - 7} fontSize="10" fill="#566172"
          fontFamily="Geist Mono">{fmtPrice(lo)}</text>
        <text x={W - PAD.right} y={H - 7} fontSize="10" fill="#566172" textAnchor="end"
          fontFamily="Geist Mono">{fmtPrice(hi)}</text>
      </svg>

      {hover && hoverCum != null && hoverInfo && (
        <div className="pointer-events-none absolute bottom-12 left-1/2 -translate-x-1/2 border border-line bg-raised px-2.5 py-1.5 text-[11px] shadow-lg shadow-black/40">
          <span className={hoverInfo.side === 'bid' ? 'text-up-text' : 'text-down-text'}>
            {hoverInfo.side.toUpperCase()}
          </span>
          <span className="mono mx-2 text-text-2">{fmtPrice(hover.price)}</span>
          <span className="mono font-semibold">{fmtQty(hoverCum)}</span>
        </div>
      )}
    </div>
  )
}
