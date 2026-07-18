import { Suspense, lazy } from 'react'
import { DepthChart } from './components/DepthChart'
import { Masthead } from './components/Masthead'
import { Annals } from './components/MetricsPanel'
import { OrderBook } from './components/OrderBook'
import { OrderEntry } from './components/OrderEntry'
import { SimControls } from './components/SimControls'
import { TickerRibbon, TradeTape } from './components/TradeTape'
import { useEngineFeed } from './hooks/useEngineFeed'

const DepthSculpture = lazy(() =>
  import('./components/DepthSculpture').then((m) => ({ default: m.DepthSculpture })),
)

function App() {
  const feed = useEngineFeed()

  return (
    <div className="flex h-dvh flex-col overflow-hidden bg-bg">
      <Masthead feed={feed} />

      <main className="grid min-h-0 flex-1 gap-px bg-line-soft p-px max-lg:grid-cols-1 max-lg:overflow-y-auto lg:grid-cols-[minmax(230px,16rem)_1fr_minmax(250px,18rem)]">
        <section className="min-h-0 max-lg:h-[70vh]">
          <OrderBook book={feed.book} />
        </section>

        <section className="flex min-h-0 flex-col gap-px">
          <div className="min-h-0 flex-[3] max-lg:h-[44vh] max-lg:flex-none">
            <Suspense
              fallback={
                <div className="tpanel flex h-full items-center justify-center">
                  <span className="tlabel">loading 3D view…</span>
                </div>
              }
            >
              <DepthSculpture />
            </Suspense>
          </div>
          <div className="min-h-0 flex-[2] max-lg:h-[34vh] max-lg:flex-none">
            <DepthChart book={feed.book} />
          </div>
        </section>

        <section className="flex min-h-0 flex-col gap-px">
          <OrderEntry />
          <SimControls />
          <div className="min-h-0 flex-[3] max-lg:h-[40vh] max-lg:flex-none">
            <TradeTape trades={feed.trades} />
          </div>
          <div className="min-h-0 flex-[2] max-lg:h-[22vh] max-lg:flex-none">
            <Annals log={feed.log} />
          </div>
        </section>
      </main>

      <footer className="flex shrink-0 items-center justify-between border-t border-line bg-panel px-3">
        <div className="min-w-0 flex-1">
          <TickerRibbon trades={feed.trades} />
        </div>
        <span className="mono ml-4 shrink-0 py-1 text-[10px] text-text-3">
          price–time priority · Rust · zero-GC
        </span>
      </footer>
    </div>
  )
}

export default App
