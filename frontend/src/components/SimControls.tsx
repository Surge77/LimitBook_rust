import { useState } from 'react'
import { startSim, stopSim } from '../lib/api'

export function SimControls() {
  const [rate, setRate] = useState(500)
  const [running, setRunning] = useState(false)

  const toggle = async () => {
    try {
      if (running) {
        await stopSim()
        setRunning(false)
      } else {
        await startSim(rate)
        setRunning(true)
      }
    } catch {
      // surface nothing critical; controls remain usable
    }
  }

  const onRate = async (next: number) => {
    setRate(next)
    if (running) await startSim(next).catch(() => undefined)
  }

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3">
      <div className="text-xs uppercase tracking-wide text-zinc-500">Simulator</div>
      <button
        onClick={toggle}
        className={`rounded px-2 py-1.5 text-sm font-semibold text-white ${
          running ? 'bg-rose-600 hover:bg-rose-500' : 'bg-emerald-600 hover:bg-emerald-500'
        }`}
      >
        {running ? 'Stop flow' : 'Start flow'}
      </button>
      <label className="text-xs text-zinc-500">
        Rate: <span className="tabular-nums text-zinc-300">{rate.toLocaleString()}</span> orders/s
        <input
          type="range"
          min={10}
          max={20000}
          step={10}
          value={rate}
          onChange={(e) => onRate(Number(e.target.value))}
          className="mt-1 w-full accent-sky-500"
        />
      </label>
    </div>
  )
}
