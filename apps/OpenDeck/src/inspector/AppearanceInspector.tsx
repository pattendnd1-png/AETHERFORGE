import type { Appearance, ControlSlot } from '../model/workspace';

interface Props { slot: ControlSlot; onUpdate: (patch: Partial<Appearance>) => void; onOpenAssets: (role: 'icon'|'background') => void }

export function AppearanceInspector({ slot, onUpdate, onOpenAssets }: Props) {
  const a = slot.appearance;
  return <div className="inspector-form appearance-grid">
    <label className="wide"><span>Title</span><input value={a.title} onChange={(e) => onUpdate({ title: e.target.value })} /></label>
    <label className="checkbox-row"><input type="checkbox" checked={a.titleVisible} onChange={(e) => onUpdate({ titleVisible: e.target.checked })} /><span>Show title</span></label>
    <label><span>Font</span><input value={a.fontFamily} onChange={(e) => onUpdate({ fontFamily: e.target.value })} /></label>
    <label><span>Size</span><input type="number" min={8} max={48} value={a.fontSize} onChange={(e) => onUpdate({ fontSize: Number(e.target.value) })} /></label>
    <label><span>Weight</span><select value={a.fontWeight} onChange={(e) => onUpdate({ fontWeight: Number(e.target.value) })}>{[300,400,500,600,700].map((v) => <option key={v}>{v}</option>)}</select></label>
    <label><span>Text</span><input type="color" value={a.textColor} onChange={(e) => onUpdate({ textColor: e.target.value })} /></label>
    <label><span>Background</span><input type="color" value={a.backgroundColor} onChange={(e) => onUpdate({ backgroundColor: e.target.value })} /></label>
    <label><span>Horizontal</span><select value={a.horizontalAlign} onChange={(e) => onUpdate({ horizontalAlign: e.target.value as Appearance['horizontalAlign'] })}><option>left</option><option>center</option><option>right</option></select></label>
    <label><span>Vertical</span><select value={a.verticalAlign} onChange={(e) => onUpdate({ verticalAlign: e.target.value as Appearance['verticalAlign'] })}><option>top</option><option>middle</option><option>bottom</option></select></label>
    <div className="asset-buttons"><button onClick={() => onOpenAssets('icon')}>Choose icon</button><button disabled={!a.iconAssetId} onClick={() => onUpdate({ iconAssetId: null })}>Clear icon</button></div>
    <div className="asset-buttons"><button onClick={() => onOpenAssets('background')}>Choose background</button><button disabled={!a.backgroundAssetId} onClick={() => onUpdate({ backgroundAssetId: null })}>Clear background</button></div>
    <label><span>Image fit</span><select value={a.fitMode} onChange={(e) => onUpdate({ fitMode: e.target.value as Appearance['fitMode'] })}><option>contain</option><option>cover</option><option>stretch</option></select></label>
    <label><span>Icon opacity</span><input type="range" min={0} max={1} step={0.05} value={a.iconOpacity} onChange={(e) => onUpdate({ iconOpacity: Number(e.target.value) })} /></label>
    <label><span>Title X</span><input type="number" min={-50} max={50} value={a.titleOffsetX} onChange={(e) => onUpdate({ titleOffsetX: Number(e.target.value) })} /></label>
    <label><span>Title Y</span><input type="number" min={-50} max={50} value={a.titleOffsetY} onChange={(e) => onUpdate({ titleOffsetY: Number(e.target.value) })} /></label>
  </div>;
}
