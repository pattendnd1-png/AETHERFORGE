import type { ActionWheel } from '../model/workspace';

interface Props {
  wheel: ActionWheel;
  onSelect: (entryId: string) => void;
  onAdd: () => void;
  onRename: (entryId: string, label: string) => void;
  onMove: (entryId: string, direction: -1 | 1) => void;
  onRemove: (entryId: string) => void;
  onRemoveWheel: () => void;
}

export function ActionWheelEditor({ wheel, onSelect, onAdd, onRename, onMove, onRemove, onRemoveWheel }: Props) {
  return <section className="dial-stack-editor action-wheel-editor" aria-label="Action Wheel editor">
    <div className="dial-stack-heading">
      <div><strong>Action Wheel</strong><span>Rotate to select · press or tap to execute</span></div>
      <span className="dial-stack-count">{wheel.activeIndex + 1} / {wheel.entries.length}</span>
    </div>
    <div className="dial-stack-list">
      {wheel.entries.map((entry, index) => {
        const active = index === wheel.activeIndex;
        return <div key={entry.id} className={`dial-stack-row${active ? ' active' : ''}`}>
          <button type="button" className="dial-stack-select" aria-label={`Select ${entry.label}`} aria-pressed={active} onClick={() => onSelect(entry.id)}><span>{index + 1}</span></button>
          <input aria-label={`Wheel entry ${index + 1} label`} value={entry.label} onChange={(event) => onRename(entry.id, event.target.value)} />
          <button type="button" aria-label={`Move ${entry.label} up`} disabled={index === 0} onClick={() => onMove(entry.id, -1)}>↑</button>
          <button type="button" aria-label={`Move ${entry.label} down`} disabled={index === wheel.entries.length - 1} onClick={() => onMove(entry.id, 1)}>↓</button>
          <button type="button" className="danger subtle" aria-label={`Remove ${entry.label}`} onClick={() => onRemove(entry.id)}>×</button>
        </div>;
      })}
    </div>
    <div className="dial-stack-actions">
      <button type="button" onClick={onAdd}>+ Add Action to Wheel</button>
      <label><span>Wheel Behavior</span><select aria-label="Wheel behavior" value={wheel.behavior} disabled><option value="rotateSelectPressExecute">Rotate select · press execute</option></select></label>
      <button type="button" className="danger subtle" onClick={onRemoveWheel}>Remove Wheel</button>
    </div>
  </section>;
}
