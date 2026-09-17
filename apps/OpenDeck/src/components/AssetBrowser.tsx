import { useEffect, useMemo, useState } from 'react';
import type { AssetRecord } from '../model/workspace';

type Role = 'icon' | 'background';
const INITIAL_VISIBLE_ASSETS = 120;
const VISIBLE_ASSET_STEP = 120;

interface Props {
  assets: AssetRecord[];
  role: Role;
  onPick: (assetId: string, role: Role) => void;
  onImport: (path: string) => Promise<void>;
  loadPreviews: (assetIds: string[]) => Promise<Record<string, string>>;
  onClose: () => void;
}

export function AssetBrowser({ assets, role, onPick, onImport, loadPreviews, onClose }: Props) {
  const [query, setQuery] = useState('');
  const [source, setSource] = useState<'all'|'imported'|'icon-pack'|'marketplace'>('all');
  const [path, setPath] = useState('');
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [visibleLimit, setVisibleLimit] = useState(INITIAL_VISIBLE_ASSETS);
  const filtered = useMemo(() => assets.filter((asset) => (source === 'all' || asset.source === source) && asset.name.toLowerCase().includes(query.toLowerCase())), [assets, source, query]);
  const visible = useMemo(() => filtered.slice(0, visibleLimit), [filtered, visibleLimit]);

  useEffect(() => {
    setVisibleLimit(INITIAL_VISIBLE_ASSETS);
  }, [query, source]);

  useEffect(() => {
    let alive = true;
    const pending = visible.filter((asset) => !Object.prototype.hasOwnProperty.call(previews, asset.id));
    if (pending.length === 0) return () => { alive = false; };
    void loadPreviews(pending.map((asset) => asset.id))
      .then((loaded) => {
        if (!alive) return;
        const additions = Object.fromEntries(pending.map((asset) => [asset.id, loaded[asset.id] ?? '']));
        setPreviews((current) => ({ ...current, ...additions }));
      })
      .catch(() => {
        if (!alive) return;
        const failed = Object.fromEntries(pending.map((asset) => [asset.id, '']));
        setPreviews((current) => ({ ...current, ...failed }));
      });
    return () => { alive = false; };
  }, [visible, loadPreviews, previews]);

  return <div className="asset-browser">
    <div className="asset-browser-head"><div><strong>{role === 'icon' ? 'Choose Icon' : 'Choose Background'}</strong><span>{assets.length} assets</span></div><button aria-label="Close assets" onClick={onClose}>×</button></div>
    <div className="asset-toolbar"><input aria-label="Search assets" placeholder="Search assets" value={query} onChange={(e) => setQuery(e.target.value)} /><select aria-label="Asset source" value={source} onChange={(e) => setSource(e.target.value as typeof source)}><option value="all">All</option><option value="imported">Imported</option><option value="icon-pack">Icon Packs</option><option value="marketplace">Marketplace</option></select></div>
    <div className="asset-import"><input aria-label="Asset path" placeholder="Image path…" value={path} onChange={(e) => setPath(e.target.value)} /><button disabled={!path.trim()} onClick={() => void onImport(path.trim()).then(() => setPath(''))}>Import</button></div>
    <div className="asset-grid">{visible.map((asset) => <button key={asset.id} className="asset-card" onClick={() => onPick(asset.id, role)} title={asset.path}>{previews[asset.id] ? <img className="asset-thumb-image" src={previews[asset.id]} alt="" /> : <span className="asset-thumb">◆</span>}<strong>{asset.name}</strong><small>{asset.source}</small></button>)}{filtered.length === 0 && <p className="muted">No matching assets.</p>}</div>
    {filtered.length > visible.length && <div className="asset-more"><span>Showing {visible.length} of {filtered.length}</span><button onClick={() => setVisibleLimit((current) => current + VISIBLE_ASSET_STEP)}>Show more</button></div>}
  </div>;
}
