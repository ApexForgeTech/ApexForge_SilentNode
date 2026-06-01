import type { Status, SeasonReport, OracleSignal, Civilization } from '../types'

const SEASON_COL: Record<string, string> = {
  Spring: '#4ade80', Summer: '#fbbf24', Autumn: '#fb923c', Winter: '#38bdf8',
}

interface BarProps { val: number; color: string }
function Bar({ val, color }: BarProps) {
  return (
    <div className="bar">
      <div className="bar-fill" style={{ width: `${(Math.min(val,1)*100).toFixed(1)}%`, background: color }} />
    </div>
  )
}

interface Props {
  status: Status | null
  season: SeasonReport | null
  oracle: OracleSignal[]
  civs: Civilization[]
  onRefresh: () => void
}

export default function LeftSidebar({ status, season, oracle, civs, onRefresh }: Props) {
  const seaCol = season ? (SEASON_COL[season.season] ?? 'var(--lavender-text)') : 'var(--t4)'

  return (
    <div className="col" style={{ height: '100%', overflow: 'hidden' }}>

      {/* Universe stats */}
      <div className="sec-head">
        <span style={{ fontSize: 9, color: 'var(--lavender-text)' }}>●</span>
        Universe
      </div>

      {!status ? (
        <div style={{ padding: '10px 12px', color: 'var(--t4)', fontSize: 11 }}>Connecting…</div>
      ) : (
        <div>
          {([
            ['Nodes',   status.node_count,       'lv'],
            ['Edges',   status.edge_count,        ''],
            ['Ghosts',  status.ghost_count,       'mt'],
            ['Fossils', status.fossil_count,      'mt'],
            ['Void',    status.void_count,        ''],
            ['Focus',   status.focus_events,      ''],
            ['Journal', status.journal_entries,   ''],
          ] as [string, number, string][]).map(([label, val, cls]) => (
            <div key={label} className="m-row">
              <span className="m-label">{label}</span>
              <span className={`m-val${cls ? ' ' + cls : ''}`}>{val}</span>
            </div>
          ))}
        </div>
      )}

      <div className="divider" />

      {/* Cognitive Season */}
      <div className="sec-head">
        <span style={{ color: seaCol, fontSize: 10 }}>◐</span>
        Season
        {season && (
          <span style={{ marginLeft: 'auto', color: seaCol, fontSize: 11, fontWeight: 600 }}>
            {season.season}
          </span>
        )}
      </div>

      {season && (
        <div style={{ padding: '6px 0' }}>
          {([
            ['Creation', season.creation_rate,    'var(--green)'],
            ['Focus',    season.focus_density,    'var(--lavender-text)'],
            ['Explore',  season.exploration_ratio, 'var(--sky)'],
            ['Revisit',  season.revisit_ratio,    seaCol],
            ['Entropy',  season.avg_entropy,      season.avg_entropy > 0.5 ? 'var(--red)' : 'var(--amber)'],
          ] as [string, number, string][]).map(([label, val, col]) => (
            <div key={label} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '3px 12px' }}>
              <span style={{ color: 'var(--t4)', fontSize: 10, width: 50, flexShrink: 0 }}>{label}</span>
              <Bar val={val} color={col} />
              <span style={{ color: col, fontSize: 10, width: 26, textAlign: 'right', flexShrink: 0 }}>
                {(val * 100).toFixed(0)}%
              </span>
            </div>
          ))}
        </div>
      )}

      <div className="divider" />

      {/* Oracle signals */}
      <div className="sec-head">
        <span style={{ color: 'var(--amber)', fontSize: 10 }}>⚡</span>
        Oracle
        <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>
          {oracle.length}
        </span>
      </div>

      <div className="scroll fill" style={{ minHeight: 0 }}>
        {oracle.length === 0 && (
          <div style={{ padding: '10px 12px', color: 'var(--t4)', fontSize: 11 }}>
            No signals
          </div>
        )}
        {oracle.slice(0, 6).map((sig, i) => {
          const col = sig.strength > 0.7 ? 'var(--red)' : sig.strength > 0.4 ? 'var(--amber)' : 'var(--green)'
          return (
            <div key={i} style={{ padding: '7px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 7, marginBottom: 3 }}>
                <div className="bar" style={{ flex: 1 }}>
                  <div className="bar-fill" style={{ width: `${(sig.strength*100).toFixed(0)}%`, background: col }} />
                </div>
                <span style={{ color: col, fontSize: 10, flexShrink: 0, fontFamily: 'var(--font-mono)' }}>
                  {sig.strength.toFixed(2)}
                </span>
              </div>
              <div style={{ color: 'var(--t2)', fontSize: 11, lineHeight: 1.4 }}>
                {sig.description.length > 54 ? sig.description.slice(0, 53) + '…' : sig.description}
              </div>
            </div>
          )
        })}

        {/* Civs */}
        {civs.length > 0 && (
          <>
            <div className="sec-head" style={{ marginTop: 4 }}>
              <span style={{ color: 'var(--teal)', fontSize: 10 }}>⬡</span>
              Civilizations
              <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{civs.length}</span>
            </div>
            {civs.slice(0, 4).map((c, i) => (
              <div key={c.id} className="m-row">
                <span className="m-label">Civ {i + 1} · {c.member_count}n</span>
                <span className="m-val" style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>
                  {c.internal_density.toFixed(2)}
                </span>
              </div>
            ))}
          </>
        )}
      </div>

      <div style={{ padding: '8px', borderTop: '1px solid var(--line)' }}>
        <button style={{ width: '100%' }} onClick={onRefresh}>↺ Refresh</button>
      </div>
    </div>
  )
}
