// Engine event log: rejections, cancels, amendments.

export function Annals({ log }: { log: string[] }) {
  return (
    <div className="tpanel flex min-h-0 flex-1 flex-col">
      <div className="flex items-center justify-between border-b border-line px-3 py-2">
        <span className="tlabel">Engine events</span>
        <span className="mono text-[10px] text-text-3">{log.length}</span>
      </div>
      <div className="mono min-h-0 flex-1 overflow-y-auto px-3 py-1.5 text-[11px] leading-[1.7] text-text-2">
        {log.length === 0 && <span className="text-text-3">no events</span>}
        {log.map((line, i) => (
          <div key={i}>{line}</div>
        ))}
      </div>
    </div>
  )
}
