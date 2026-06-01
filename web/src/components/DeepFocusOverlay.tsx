import type { SeasonReport, TectonicData } from '../types'

export default function DeepFocusOverlay({
  season,
  tectonics,
  onExit,
}: {
  season: SeasonReport | null
  tectonics: TectonicData | null
  onExit: () => void
}) {
  return (
    <div className="deep-focus">
      <button onClick={onExit}>Exit</button>
      <div>
        <small>{season?.season ?? 'Silent'} focus field</small>
        <h1>{tectonics?.epicenter_preview || 'Deep Focus'}</h1>
        <p>{tectonics?.description || 'The interface is reduced. Stay with the work.'}</p>
      </div>
    </div>
  )
}
