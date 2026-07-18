// Feed controller: prefers the live Rust gateway (WebSocket + REST) when the app
// is served next to one; otherwise runs the in-browser simulacrum. Components
// subscribe to a single stream and route orders here without caring which.

import type { NewOrderBody, ServerMessage } from '../types'
import { cancelOrder, submitOrder, type OrderAck } from './api'
import { LocalBourse } from './sim'

export type FeedMode = 'gateway' | 'simulacrum'

type Listener = (msg: ServerMessage) => void
type ModeListener = (mode: FeedMode, connected: boolean) => void

const WS_CONNECT_TIMEOUT_MS = 2500

const isLocalhost = ['localhost', '127.0.0.1'].includes(window.location.hostname)

class FeedController {
  private listeners = new Set<Listener>()
  private modeListeners = new Set<ModeListener>()
  private sim: LocalBourse | null = null
  private started = false
  mode: FeedMode = 'simulacrum'
  connected = false

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn)
    if (!this.started) {
      this.started = true
      if (isLocalhost) this.tryGateway()
      else this.startSim()
    }
    return () => this.listeners.delete(fn)
  }

  onMode(fn: ModeListener): () => void {
    this.modeListeners.add(fn)
    fn(this.mode, this.connected)
    return () => this.modeListeners.delete(fn)
  }

  async submit(body: NewOrderBody): Promise<OrderAck> {
    if (this.mode === 'gateway') return submitOrder(body)
    return this.sim!.submit(body)
  }

  async cancel(id: number): Promise<void> {
    if (this.mode === 'gateway') return cancelOrder(id)
    this.sim!.cancel(id)
  }

  setIntensity(rate: number): void {
    this.sim?.setIntensity(rate)
  }

  // ── internals ──────────────────────────────────────────────────────────

  private broadcast(msg: ServerMessage): void {
    for (const fn of this.listeners) fn(msg)
  }

  private setMode(mode: FeedMode, connected: boolean): void {
    this.mode = mode
    this.connected = connected
    for (const fn of this.modeListeners) fn(mode, connected)
  }

  private startSim(): void {
    if (this.sim) return
    this.sim = new LocalBourse()
    this.sim.start((msg) => this.broadcast(msg))
    this.setMode('simulacrum', true)
  }

  private tryGateway(): void {
    const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
    const ws = new WebSocket(`${proto}://${window.location.host}/ws`)
    const giveUp = setTimeout(() => {
      ws.close()
      this.startSim()
    }, WS_CONNECT_TIMEOUT_MS)

    ws.onopen = () => {
      clearTimeout(giveUp)
      this.setMode('gateway', true)
    }
    ws.onmessage = (e) => {
      try {
        this.broadcast(JSON.parse(e.data as string) as ServerMessage)
      } catch {
        // ignore malformed frame
      }
    }
    ws.onclose = () => {
      clearTimeout(giveUp)
      // Whether the live connection dropped or never opened, fall back to the
      // simulacrum rather than a dead page.
      this.startSim()
    }
    ws.onerror = () => ws.close()
  }
}

export const feed = new FeedController()
