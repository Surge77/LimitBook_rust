// Connects to the gateway WebSocket, maintains live book/trade/event state, and computes
// trades/sec + orders/sec. Reconnects with exponential backoff (per project WS pattern).

import { useEffect, useReducer, useRef } from 'react'
import type { BookMsg, ServerMessage, TradeMsg } from '../types'

const MAX_TRADES = 60
const MAX_LOG = 40
const MAX_BACKOFF_MS = 15_000

export interface FeedState {
  connected: boolean
  book: BookMsg | null
  trades: TradeMsg[]
  log: string[]
  tradesPerSec: number
  ordersPerSec: number
}

type Action =
  | { kind: 'connected' }
  | { kind: 'disconnected' }
  | { kind: 'message'; msg: ServerMessage }
  | { kind: 'rates'; tradesPerSec: number; ordersPerSec: number }

const initialState: FeedState = {
  connected: false,
  book: null,
  trades: [],
  log: [],
  tradesPerSec: 0,
  ordersPerSec: 0,
}

function reducer(state: FeedState, action: Action): FeedState {
  switch (action.kind) {
    case 'connected':
      return { ...state, connected: true }
    case 'disconnected':
      return { ...state, connected: false }
    case 'rates':
      return { ...state, tradesPerSec: action.tradesPerSec, ordersPerSec: action.ordersPerSec }
    case 'message':
      return applyMessage(state, action.msg)
    default:
      return state
  }
}

function applyMessage(state: FeedState, msg: ServerMessage): FeedState {
  switch (msg.type) {
    case 'book':
      return { ...state, book: msg }
    case 'trade':
      return { ...state, trades: [msg, ...state.trades].slice(0, MAX_TRADES) }
    case 'order_rejected':
      return logLine(state, `✗ #${msg.id} rejected: ${msg.reason}`)
    case 'order_canceled':
      return logLine(state, `⊘ #${msg.id} canceled (rem ${msg.remaining})`)
    case 'order_amended':
      return logLine(state, `± #${msg.id} amended${msg.repriced ? ' (repriced)' : ''}`)
    case 'order_accepted':
      return state // too noisy to log; counted for orders/sec
    default:
      return state
  }
}

function logLine(state: FeedState, line: string): FeedState {
  return { ...state, log: [line, ...state.log].slice(0, MAX_LOG) }
}

function wsUrl(): string {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  return `${proto}://${window.location.host}/ws`
}

export function useEngineFeed(): FeedState {
  const [state, dispatch] = useReducer(reducer, initialState)
  const tradeCount = useRef(0)
  const orderCount = useRef(0)

  useEffect(() => {
    let ws: WebSocket | null = null
    let backoff = 1000
    let closed = false
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined

    const connect = () => {
      if (closed) return
      ws = new WebSocket(wsUrl())
      ws.onopen = () => {
        backoff = 1000
        dispatch({ kind: 'connected' })
      }
      ws.onmessage = (e) => {
        try {
          const msg = JSON.parse(e.data as string) as ServerMessage
          if (msg.type === 'trade') tradeCount.current += msg.quantity
          if (msg.type === 'order_accepted') orderCount.current += 1
          dispatch({ kind: 'message', msg })
        } catch {
          // ignore malformed frame
        }
      }
      ws.onclose = () => {
        dispatch({ kind: 'disconnected' })
        if (closed) return
        backoff = Math.min(backoff * 2, MAX_BACKOFF_MS)
        reconnectTimer = setTimeout(connect, backoff)
      }
      ws.onerror = () => ws?.close()
    }

    const rateTimer = setInterval(() => {
      dispatch({ kind: 'rates', tradesPerSec: tradeCount.current, ordersPerSec: orderCount.current })
      tradeCount.current = 0
      orderCount.current = 0
    }, 1000)

    connect()

    return () => {
      closed = true
      clearInterval(rateTimer)
      if (reconnectTimer) clearTimeout(reconnectTimer)
      ws?.close()
    }
  }, [])

  return state
}
