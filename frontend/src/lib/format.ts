// Display helpers. Prices on the wire are integer ticks; 1 tick = 0.01 quote units.

export const TICK = 0.01

export function fmtPrice(ticks: number | null | undefined): string {
  if (ticks == null) return '—'
  return (ticks * TICK).toFixed(2)
}

export function fmtQty(qty: number): string {
  return qty.toLocaleString('en-US')
}

export function fmtInt(n: number): string {
  return n.toLocaleString('en-US')
}
