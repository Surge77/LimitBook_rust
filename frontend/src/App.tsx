import { DepthChart } from './components/DepthChart'
import { MetricsPanel } from './components/MetricsPanel'
import { OrderBook } from './components/OrderBook'
import { OrderEntry } from './components/OrderEntry'
import { SimControls } from './components/SimControls'
import { TradeTape } from './components/TradeTape'
import { useEngineFeed } from './hooks/useEngineFeed'

function App() {
  const feed = useEngineFeed()

  return (
    <div className="mx-auto flex h-full max-w-[1600px] flex-col gap-3 p-3">
      <header className="flex items-baseline justify-between">
        <h1 className="text-xl font-bold text-emerald-400">
          LimitBook <span className="text-zinc-500">⚡ matching engine</span>
        </h1>
        <span className="text-xs text-zinc-600">price-time priority · Rust · zero-GC</span>
      </header>

      <div className="grid flex-1 grid-cols-12 gap-3 overflow-hidden">
        <section className="col-span-3 min-h-0">
          <OrderBook book={feed.book} />
        </section>

        <section className="col-span-6 flex min-h-0 flex-col gap-3">
          <div className="h-1/2 min-h-0">
            <DepthChart book={feed.book} />
          </div>
          <div className="h-1/2 min-h-0">
            <TradeTape trades={feed.trades} />
          </div>
        </section>

        <section className="col-span-3 flex min-h-0 flex-col gap-3 overflow-y-auto">
          <MetricsPanel feed={feed} />
          <OrderEntry />
          <SimControls />
          <EventLog log={feed.log} />
        </section>
      </div>
    </div>
  )
}

function EventLog({ log }: { log: string[] }) {
  return (
    <div className="flex min-h-0 flex-1 flex-col rounded-lg border border-zinc-800 bg-zinc-950 p-3">
      <div className="pb-1 text-xs uppercase tracking-wide text-zinc-500">Events</div>
      <div className="flex-1 overflow-y-auto text-xs tabular-nums text-zinc-400">
        {log.length === 0 && <div className="text-zinc-600">—</div>}
        {log.map((line, i) => (
          <div key={i}>{line}</div>
        ))}
      </div>
    </div>
  )
}

export default App
