// Flow throttle for synthetic order flow. Drives the in-browser sim directly;
// against a live gateway it calls the /sim endpoints instead.

import { useState } from 'react'
import { startSim, stopSim } from '../lib/api'
import { feed } from '../lib/feed'

const RATES = [
  { label: 'Calm', rate: 12 },
  { label: 'Active', rate: 40 },
  { label: 'Busy', rate: 120 },
  { label: 'Stress', rate: 240 },
] as const

export function SimControls() {
  const [active, setActive] = useState(1)

  const choose = async (i: number) => {
    setActive(i)
    const { rate } = RATES[i]
    if (feed.mode === 'gateway') {
      try {
        await startSim(rate)
      } catch {
        await stopSim().catch(() => {})
      }
    } else {
      feed.setIntensity(rate)
    }
  }

  return (
    <div className="tpanel flex flex-col gap-2 p-3">
      <div className="flex items-center justify-between">
        <span className="tlabel">Market activity</span>
        <span className="mono text-[10px] text-text-3">{RATES[active].rate}/s</span>
      </div>
      <div className="grid grid-cols-4 gap-1">
        {RATES.map((r, i) => (
          <button
            key={r.label}
            onClick={() => choose(i)}
            className={`py-1.5 text-[11px] font-medium transition-colors ${
              active === i
                ? 'bg-amber/15 text-amber outline outline-1 outline-amber/50'
                : 'border border-line bg-raised text-text-2 hover:text-text'
            }`}
          >
            {r.label}
          </button>
        ))}
      </div>
    </div>
  )
}
