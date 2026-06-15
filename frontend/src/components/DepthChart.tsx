import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import type { BookMsg } from '../types'
import { fmtPrice } from '../lib/format'

interface DepthChartProps {
  book: BookMsg | null
}

interface Point {
  price: number
  bid?: number
  ask?: number
}

/** Build cumulative depth curves: bids accumulate from best downward, asks from best upward. */
function buildSeries(book: BookMsg | null): Point[] {
  if (!book) return []
  const points: Point[] = []
  let cum = 0
  // Bids: best (highest) first; reverse so the x-axis runs low→high price.
  const bidPts: Point[] = []
  for (const l of book.bids) {
    cum += l.quantity
    bidPts.push({ price: l.price, bid: cum })
  }
  points.push(...bidPts.reverse())
  cum = 0
  for (const l of book.asks) {
    cum += l.quantity
    points.push({ price: l.price, ask: cum })
  }
  return points
}

export function DepthChart({ book }: DepthChartProps) {
  const data = buildSeries(book)
  return (
    <div className="flex h-full flex-col rounded-lg border border-zinc-800 bg-zinc-950 p-2">
      <div className="px-1 pb-1 text-xs uppercase tracking-wide text-zinc-500">Depth</div>
      <div className="flex-1">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 4, right: 8, left: 0, bottom: 0 }}>
            <XAxis
              dataKey="price"
              type="number"
              domain={['dataMin', 'dataMax']}
              tickFormatter={fmtPrice}
              stroke="#52525b"
              fontSize={11}
            />
            <YAxis stroke="#52525b" fontSize={11} width={40} />
            <Tooltip
              contentStyle={{ background: '#18181b', border: '1px solid #3f3f46', fontSize: 12 }}
              labelFormatter={(v) => `Price ${fmtPrice(Number(v))}`}
            />
            <Area
              type="stepAfter"
              dataKey="bid"
              stroke="#34d399"
              fill="#34d399"
              fillOpacity={0.2}
              isAnimationActive={false}
              connectNulls
            />
            <Area
              type="stepBefore"
              dataKey="ask"
              stroke="#fb7185"
              fill="#fb7185"
              fillOpacity={0.2}
              isAnimationActive={false}
              connectNulls
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}
