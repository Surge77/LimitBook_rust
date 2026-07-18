// Live book/trade/event state from the feed controller (gateway or simulacrum),
// plus trades/sec + orders/sec rates and a rolling last-trade-price series.

import { useEffect, useReducer, useRef } from 'react'
import type { BookMsg, ServerMessage, TradeMsg } from '../types'
import { feed, type FeedMode } from '../lib/feed'

const MAX_TRADES = 48
const MAX_LOG = 40
const MAX_PRICES = 160

export interface FeedState {
  mode: FeedMode
  connected: boolean
  book: BookMsg | null
  trades: TradeMsg[]
  prices: number[]
  log: string[]
  tradesPerSec: number
  ordersPerSec: number
}

type Action =
  | { kind: 'mode'; mode: FeedMode; connected: boolean }
  | { kind: 'message'; msg: ServerMessage }
  | { kind: 'rates'; tradesPerSec: number; ordersPerSec: number }

const initialState: FeedState = {
  mode: 'simulacrum',
  connected: false,
  book: null,
  trades: [],
  prices: [],
  log: [],
  tradesPerSec: 0,
  ordersPerSec: 0,
}

function reducer(state: FeedState, action: Action): FeedState {
  switch (action.kind) {
    case 'mode':
      return { ...state, mode: action.mode, connected: action.connected }
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
      return {
        ...state,
        trades: [msg, ...state.trades].slice(0, MAX_TRADES),
        prices: [...state.prices, msg.price].slice(-MAX_PRICES),
      }
    case 'order_rejected':
      return logLine(state, `✗ #${msg.id} rejected — ${msg.reason}`)
    case 'order_canceled':
      return logLine(state, `⊘ #${msg.id} withdrawn (rem ${msg.remaining})`)
    case 'order_amended':
      return logLine(state, `± #${msg.id} amended${msg.repriced ? ' (repriced)' : ''}`)
    case 'order_accepted':
      return state // counted for orders/sec only
    default:
      return state
  }
}

function logLine(state: FeedState, line: string): FeedState {
  return { ...state, log: [line, ...state.log].slice(0, MAX_LOG) }
}

export function useEngineFeed(): FeedState {
  const [state, dispatch] = useReducer(reducer, initialState)
  const tradeCount = useRef(0)
  const orderCount = useRef(0)

  useEffect(() => {
    const unsubMsg = feed.subscribe((msg) => {
      if (msg.type === 'trade') tradeCount.current += 1
      if (msg.type === 'order_accepted') orderCount.current += 1
      dispatch({ kind: 'message', msg })
    })
    const unsubMode = feed.onMode((mode, connected) =>
      dispatch({ kind: 'mode', mode, connected }),
    )
    const rateTimer = setInterval(() => {
      dispatch({
        kind: 'rates',
        tradesPerSec: tradeCount.current,
        ordersPerSec: orderCount.current,
      })
      tradeCount.current = 0
      orderCount.current = 0
    }, 1000)

    return () => {
      unsubMsg()
      unsubMode()
      clearInterval(rateTimer)
    }
  }, [])

  return state
}
