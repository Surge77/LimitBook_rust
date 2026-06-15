// REST client for the gateway. All writes are async (the engine acks intake; fills arrive on
// the WebSocket), so these resolve on 202.

import type { NewOrderBody } from '../types'

async function postJson<T>(url: string, body: unknown): Promise<T> {
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  })
  const data = (await res.json().catch(() => ({}))) as T
  if (!res.ok) {
    throw new ApiError(res.status, data)
  }
  return data
}

export class ApiError extends Error {
  readonly status: number
  readonly body: unknown

  constructor(status: number, body: unknown) {
    super(`request failed with status ${status}`)
    this.name = 'ApiError'
    this.status = status
    this.body = body
  }
}

export interface OrderAck {
  id: number
  accepted: boolean
}

export function submitOrder(body: NewOrderBody): Promise<OrderAck> {
  return postJson<OrderAck>('/orders', body)
}

export async function cancelOrder(id: number): Promise<void> {
  const res = await fetch(`/orders/${id}`, { method: 'DELETE' })
  if (!res.ok) throw new ApiError(res.status, await res.json().catch(() => ({})))
}

export function startSim(rate: number): Promise<{ running: boolean; rate: number }> {
  return postJson('/sim/start', { rate })
}

export function stopSim(): Promise<{ running: boolean }> {
  return postJson('/sim/stop', {})
}
