// Order ticket, exchange style: green/red buy-sell tabs, tick-stepped price
// input, submits to whichever engine is quoting (gateway or in-browser sim).

import { useState } from 'react'
import { feed } from '../lib/feed'
import type { OrderTypeWire, Side } from '../types'

const inputCls =
  'mono w-full border border-line bg-bg px-2 py-1.5 text-[13px] text-text outline-none transition-colors focus:border-amber/70'

export function OrderEntry() {
  const [side, setSide] = useState<Side>('buy')
  const [orderType, setOrderType] = useState<OrderTypeWire>('limit')
  const [price, setPrice] = useState('187.40')
  const [qty, setQty] = useState('25')
  const [note, setNote] = useState<string | null>(null)

  const submit = async () => {
    const quantity = Number(qty)
    const priceTicks = Math.round(Number(price) * 100)
    try {
      const ack = await feed.submit({
        side,
        order_type: orderType,
        quantity,
        ...(orderType === 'limit' ? { price: priceTicks } : {}),
      })
      setNote(ack.accepted ? `order #${ack.id} accepted` : `order #${ack.id} rejected`)
    } catch {
      setNote('order rejected by engine')
    }
  }

  const isBuy = side === 'buy'
  return (
    <div className="tpanel flex flex-col gap-2.5 p-3">
      <div className="flex items-center justify-between">
        <span className="tlabel">Order ticket</span>
        <span className="mono text-[10px] text-text-3">LMB/USD</span>
      </div>

      <div className="grid grid-cols-2 gap-1">
        <button
          onClick={() => setSide('buy')}
          className={`py-1.5 text-[12px] font-semibold transition-colors ${
            isBuy
              ? 'bg-up text-white'
              : 'border border-line bg-raised text-text-2 hover:text-up-text'
          }`}
        >
          Buy
        </button>
        <button
          onClick={() => setSide('sell')}
          className={`py-1.5 text-[12px] font-semibold transition-colors ${
            !isBuy
              ? 'bg-down text-white'
              : 'border border-line bg-raised text-text-2 hover:text-down-text'
          }`}
        >
          Sell
        </button>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <label className="flex flex-col gap-1">
          <span className="text-[10px] font-medium uppercase tracking-wide text-text-3">Type</span>
          <select
            value={orderType}
            onChange={(e) => setOrderType(e.target.value as OrderTypeWire)}
            className={inputCls}
          >
            <option value="limit">Limit</option>
            <option value="market">Market</option>
          </select>
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-[10px] font-medium uppercase tracking-wide text-text-3">
            Quantity
          </span>
          <input type="number" min="1" value={qty} onChange={(e) => setQty(e.target.value)}
            className={inputCls} />
        </label>
      </div>

      {orderType === 'limit' && (
        <label className="flex flex-col gap-1">
          <span className="text-[10px] font-medium uppercase tracking-wide text-text-3">
            Price (USD)
          </span>
          <input type="number" step="0.01" value={price} onChange={(e) => setPrice(e.target.value)}
            className={inputCls} />
        </label>
      )}

      <button
        onClick={submit}
        className={`py-2 text-[13px] font-semibold text-white transition-opacity hover:opacity-90 ${
          isBuy ? 'bg-up' : 'bg-down'
        }`}
      >
        {isBuy ? 'Buy LMB' : 'Sell LMB'}
      </button>

      {note && <div className="mono text-center text-[10.5px] text-text-3">{note}</div>}
    </div>
  )
}
