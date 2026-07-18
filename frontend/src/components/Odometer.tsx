// Mechanical rolling-digit price display, like a departure board carved into the
// masthead. Each digit is a vertical rail of 0–9 translated to the active glyph.

const DIGITS = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']

interface OdometerProps {
  value: string // pre-formatted, e.g. "187.42"
  className?: string
}

export function Odometer({ value, className = '' }: OdometerProps) {
  return (
    <span
      className={`inline-flex overflow-hidden align-baseline ${className}`}
      style={{ height: '1em', lineHeight: '1em' }}
      aria-label={value}
    >
      {value.split('').map((ch, i) =>
        /\d/.test(ch) ? (
          <span key={i} className="relative inline-block" style={{ width: '0.62em' }}>
            <span
              className="odo-col absolute left-0 top-0 flex flex-col items-center"
              style={{ transform: `translateY(-${Number(ch)}em)` }}
              aria-hidden
            >
              {DIGITS.map((d) => (
                <span key={d} style={{ height: '1em', lineHeight: '1em' }}>
                  {d}
                </span>
              ))}
            </span>
          </span>
        ) : (
          <span key={i} style={{ height: '1em', lineHeight: '1em' }}>
            {ch}
          </span>
        ),
      )}
    </span>
  )
}
