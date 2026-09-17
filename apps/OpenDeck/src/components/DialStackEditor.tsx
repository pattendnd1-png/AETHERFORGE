import type { DialStack } from '../model/workspace';

interface Props {
  stack: DialStack;
  onSelect: (entryId: string) => void;
  onAdd: () => void;
  onRename: (entryId: string, label: string) => void;
  onMove: (entryId: string, direction: -1 | 1) => void;
  onRemove: (entryId: string) => void;
  onRemoveStack: () => void;
}

export function DialStackEditor({ stack, onSelect, onAdd, onRename, onMove, onRemove, onRemoveStack }: Props) {
  return <section className="dial-stack-editor" aria-label="Dial Stack editor">
    <div className="dial-stack-heading">
      <div><strong>Dial Stack</strong><span>Press cycles the active entry</span></div>
      <span className="dial-stack-count">{stack.activeIndex + 1} / {stack.entries.length}</span>
    </div>
    <div className="dial-stack-list">
      {stack.entries.map((entry, index) => {
        const active = index === stack.activeIndex;
        return <div key={entry.id} className={`dial-stack-row${active ? ' active' : ''}`}>
          <button type="button" className="dial-stack-select" aria-label={`Select ${entry.label}`} aria-pressed={active} onClick={() => onSelect(entry.id)}>
            <span>{index + 1}</span>
          </button>
          <input aria-label={`Stack entry ${index + 1} label`} value={entry.label} onChange={(event) => onRename(entry.id, event.target.value)} />
          <button type="button" aria-label={`Move ${entry.label} up`} disabled={index === 0} onClick={() => onMove(entry.id, -1)}>↑</button>
          <button type="button" aria-label={`Move ${entry.label} down`} disabled={index === stack.entries.length - 1} onClick={() => onMove(entry.id, 1)}>↓</button>
          <button type="button" className="danger subtle" aria-label={`Remove ${entry.label}`} onClick={() => onRemove(entry.id)}>×</button>
        </div>;
      })}
    </div>
    <div className="dial-stack-actions">
      <button type="button" onClick={onAdd}>+ Add Action to Stack</button>
      <label><span>Stack Behavior</span><select aria-label="Stack behavior" value={stack.behavior} disabled><option value="pressCycle">Press to cycle</option></select></label>
      <button type="button" className="danger subtle" onClick={onRemoveStack}>Remove Stack</button>
    </div>
  </section>;
}
