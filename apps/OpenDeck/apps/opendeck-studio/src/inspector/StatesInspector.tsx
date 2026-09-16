import type { Appearance, AppearanceOverride, ControlSlot } from '../model/workspace';

interface Props {
  slot: ControlSlot;
  previewState: 'default' | 'active';
  onPreviewState: (state: 'default' | 'active') => void;
  onUpdate: (patch: AppearanceOverride) => void;
  onReset: () => void;
  onCopyDefault: () => void;
  onOpenAssets: (role: 'icon'|'background') => void;
}

export function StatesInspector({ slot, previewState, onPreviewState, onUpdate, onReset, onCopyDefault, onOpenAssets }: Props) {
  const active = slot.states.active ?? {};
  const value = <K extends keyof Appearance>(key: K): Appearance[K] => active[key] === undefined ? slot.appearance[key] : active[key] as Appearance[K];
  const activeIcon = value('iconAssetId');
  const activeBackground = value('backgroundAssetId');

  return <div className="inspector-form appearance-grid">
    <div className="binding-title wide"><strong>Active state</strong><span>{Object.keys(active).length ? `${Object.keys(active).length} overrides` : 'Uses default appearance'}</span></div>
    <div className="button-row wide state-preview-toggle">
      <button className={previewState === 'default' ? 'active' : ''} aria-label="Preview default state" onClick={() => onPreviewState('default')}>Default preview</button>
      <button className={previewState === 'active' ? 'active' : ''} aria-label="Preview active state" onClick={() => onPreviewState('active')}>Active preview</button>
    </div>
    <label className="wide"><span>Title</span><input value={String(value('title'))} onChange={(e) => onUpdate({ title: e.target.value })} /></label>
    <label className="checkbox-row"><input type="checkbox" checked={Boolean(value('titleVisible'))} onChange={(e) => onUpdate({ titleVisible: e.target.checked })} /><span>Show title</span></label>
    <label><span>Font</span><input value={String(value('fontFamily'))} onChange={(e) => onUpdate({ fontFamily: e.target.value })} /></label>
    <label><span>Size</span><input type="number" min={8} max={48} value={Number(value('fontSize'))} onChange={(e) => onUpdate({ fontSize: Number(e.target.value) })} /></label>
    <label><span>Weight</span><select value={Number(value('fontWeight'))} onChange={(e) => onUpdate({ fontWeight: Number(e.target.value) })}>{[300,400,500,600,700].map((weight) => <option key={weight}>{weight}</option>)}</select></label>
    <label><span>Text</span><input type="color" value={String(value('textColor'))} onChange={(e) => onUpdate({ textColor: e.target.value })} /></label>
    <label><span>Background</span><input type="color" value={String(value('backgroundColor'))} onChange={(e) => onUpdate({ backgroundColor: e.target.value })} /></label>
    <label><span>Horizontal</span><select value={String(value('horizontalAlign'))} onChange={(e) => onUpdate({ horizontalAlign: e.target.value as Appearance['horizontalAlign'] })}><option>left</option><option>center</option><option>right</option></select></label>
    <label><span>Vertical</span><select value={String(value('verticalAlign'))} onChange={(e) => onUpdate({ verticalAlign: e.target.value as Appearance['verticalAlign'] })}><option>top</option><option>middle</option><option>bottom</option></select></label>
    <div className="asset-buttons"><button onClick={() => onOpenAssets('icon')}>Choose active icon</button><button disabled={activeIcon === null} onClick={() => onUpdate({ iconAssetId: null })}>No active icon</button></div>
    <div className="asset-buttons"><button onClick={() => onOpenAssets('background')}>Choose active background</button><button disabled={activeBackground === null} onClick={() => onUpdate({ backgroundAssetId: null })}>No active background</button></div>
    <label><span>Image fit</span><select value={String(value('fitMode'))} onChange={(e) => onUpdate({ fitMode: e.target.value as Appearance['fitMode'] })}><option>contain</option><option>cover</option><option>stretch</option></select></label>
    <label><span>Icon opacity</span><input type="range" min={0} max={1} step={0.05} value={Number(value('iconOpacity'))} onChange={(e) => onUpdate({ iconOpacity: Number(e.target.value) })} /></label>
    <label><span>Title X</span><input type="number" min={-50} max={50} value={Number(value('titleOffsetX'))} onChange={(e) => onUpdate({ titleOffsetX: Number(e.target.value) })} /></label>
    <label><span>Title Y</span><input type="number" min={-50} max={50} value={Number(value('titleOffsetY'))} onChange={(e) => onUpdate({ titleOffsetY: Number(e.target.value) })} /></label>
    <div className="button-row wide"><button onClick={onCopyDefault}>Copy Default</button><button className="danger subtle" onClick={onReset}>Reset Override</button></div>
  </div>;
}
