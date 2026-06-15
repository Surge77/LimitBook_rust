import { useState } from 'react'
import { ApiError, cancelOrder, submitOrder } from '../lib/api'
import type { OrderTypeWire, Side } from '../types'

const ORDER_TYPES: { value: OrderTypeWire; label: string }[] = [
  { value: 'limit', label: 'Limit' },
  { value: 'market', label: 'Market' },
  { value: 'immediate_or_cancel', label: 'IOC' },
  { value: 'fill_or_kill', label: 'FOK' },
  { value: 'post_only', label: 'Post-Only' },
]

const needsPrice = (t: OrderTypeWire) => t === 'limit' || t === 'post_only'

export function OrderEntry() {
  const [side, setSide] = useState<Side>('buy')
  const [orderType, setOrderType] = useState<OrderTypeWire>('limit')
  const [price, setPrice] = useState('100.00')
  const [qty, setQty] = useState('10')
  const [cancelId, setCancelId] = useState('')
  const [status, setStatus] = useState<string | null>(null)

  const submit = async () => {
    setStatus(null)
    try {
      const body = {
        side,
        order_type: orderType,
        quantity: Number(qty),
        ...(needsPrice(orderType) ? { price: Math.round(Number(price) * 100) } : {}),
      }
      const ack = await submitOrder(body)
      setStatus(`✓ submitted #${ack.id}`)
    } catch (e) {
      setStatus(e instanceof ApiError ? `✗ rejected (${e.status})` : '✗ network error')
    }
  }

  const cancel = async () => {
    setStatus(null)
    const id = Number(cancelId)
    if (!id) return
    try {
      await cancelOrder(id)
      setStatus(`⊘ cancel sent #${id}`)
    } catch (e) {
      setStatus(e instanceof ApiError ? `✗ cancel failed (${e.status})` : '✗ network error')
    }
  }

  const inputCls = 'w-full rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm tabular-nums'

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3">
      <div className="text-xs uppercase tracking-wide text-zinc-500">Order Entry</div>
      <div className="grid grid-cols-2 gap-1">
        {(['buy', 'sell'] as Side[]).map((s) => (
          <button
            key={s}
            onClick={() => setSide(s)}
            className={`rounded px-2 py-1 text-sm font-semibold uppercase ${
              side === s
                ? s === 'buy'
                  ? 'bg-emerald-600 text-white'
                  : 'bg-rose-600 text-white'
                : 'bg-zinc-800 text-zinc-400'
            }`}
          >
            {s}
          </button>
        ))}
      </div>
      <select
        value={orderType}
        onChange={(e) => setOrderType(e.target.value as OrderTypeWire)}
        className={inputCls}
      >
        {ORDER_TYPES.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      {needsPrice(orderType) && (
        <label className="text-xs text-zinc-500">
          Price
          <input className={inputCls} value={price} onChange={(e) => setPrice(e.target.value)} />
        </label>
      )}
      <label className="text-xs text-zinc-500">
        Quantity
        <input className={inputCls} value={qty} onChange={(e) => setQty(e.target.value)} />
      </label>
      <button
        onClick={submit}
        className="rounded bg-sky-600 px-2 py-1.5 text-sm font-semibold text-white hover:bg-sky-500"
      >
        Submit
      </button>
      <div className="mt-1 flex gap-1">
        <input
          className={inputCls}
          placeholder="cancel id"
          value={cancelId}
          onChange={(e) => setCancelId(e.target.value)}
        />
        <button
          onClick={cancel}
          className="rounded bg-zinc-700 px-2 py-1 text-sm text-zinc-200 hover:bg-zinc-600"
        >
          Cancel
        </button>
      </div>
      {status && <div className="text-xs text-zinc-400">{status}</div>}
    </div>
  )
}
