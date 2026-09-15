import type { AppearanceOverride, ControlSlot } from '../model/workspace';

interface Props { slot: ControlSlot; onUpdate: (patch: AppearanceOverride) => void; onReset: () => void; onCopyDefault: () => void }

export function StatesInspector({ slot, onUpdate, onReset, onCopyDefault }: Props) {
  const active = slot.states.active ?? {};
  return <div className="inspector-form">
    <div className="binding-title"><strong>Active state</strong><span>{Object.keys(active).length ? `${Object.keys(active).length} overrides` : 'Uses default appearance'}</span></div>
    <label><span>Title override</span><input value={String(active.title ?? '')} placeholder={slot.appearance.title} onChange={(e) => onUpdate({ title: e.target.value })} /></label>
    <label><span>Text color</span><input type="color" value={String(active.textColor ?? slot.appearance.textColor)} onChange={(e) => onUpdate({ textColor: e.target.value })} /></label>
    <label><span>Background</span><input type="color" value={String(active.backgroundColor ?? slot.appearance.backgroundColor)} onChange={(e) => onUpdate({ backgroundColor: e.target.value })} /></label>
    <div className="button-row"><button onClick={onCopyDefault}>Copy Default</button><button className="danger subtle" onClick={onReset}>Reset Override</button></div>
  </div>;
}
