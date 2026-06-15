// Wire types mirroring the gateway's `ServerMessage` (serde tag = "type", snake_case).

export type Side = 'buy' | 'sell'

export type OrderTypeWire =
  | 'limit'
  | 'market'
  | 'immediate_or_cancel'
  | 'fill_or_kill'
  | 'post_only'
  | 'stop'
  | 'stop_limit'

export interface Level {
  price: number
  quantity: number
}

export interface BookMsg {
  type: 'book'
  bids: Level[]
  asks: Level[]
  seq: number
  best_bid: number | null
  best_ask: number | null
  spread: number | null
}

export interface TradeMsg {
  type: 'trade'
  seq: number
  price: number
  quantity: number
  taker_side: Side
  taker_order: number
  maker_order: number
}

export interface OrderAcceptedMsg {
  type: 'order_accepted'
  id: number
}

export interface OrderRejectedMsg {
  type: 'order_rejected'
  id: number
  reason: string
}

export interface OrderCanceledMsg {
  type: 'order_canceled'
  id: number
  remaining: number
}

export interface OrderAmendedMsg {
  type: 'order_amended'
  id: number
  quantity: number
  price: number | null
  repriced: boolean
}

export type ServerMessage =
  | BookMsg
  | TradeMsg
  | OrderAcceptedMsg
  | OrderRejectedMsg
  | OrderCanceledMsg
  | OrderAmendedMsg

export interface NewOrderBody {
  side: Side
  order_type: OrderTypeWire
  price?: number
  stop_price?: number
  quantity: number
  account?: number
}
