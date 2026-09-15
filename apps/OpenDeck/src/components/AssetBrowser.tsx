import { useMemo, useState } from 'react';
import type { AssetRecord } from '../model/workspace';

type Role = 'icon' | 'background';
interface Props {
  assets: AssetRecord[];
  role: Role;
  onPick: (assetId: string, role: Role) => void;
  onImport: (path: string) => Promise<void>;
  onClose: () => void;
}

export function AssetBrowser({ assets, role, onPick, onImport, onClose }: Props) {
  const [query, setQuery] = useState('');
  const [source, setSource] = useState<'all'|'imported'|'icon-pack'|'marketplace'>('all');
  const [path, setPath] = useState('');
  const filtered = useMemo(() => assets.filter((asset) => (source === 'all' || asset.source === source) && asset.name.toLowerCase().includes(query.toLowerCase())), [assets, source, query]);
  return <div className="asset-browser">
    <div className="asset-browser-head"><div><strong>{role === 'icon' ? 'Choose Icon' : 'Choose Background'}</strong><span>{assets.length} assets</span></div><button aria-label="Close assets" onClick={onClose}>×</button></div>
    <div className="asset-toolbar"><input aria-label="Search assets" placeholder="Search assets" value={query} onChange={(e) => setQuery(e.target.value)} /><select aria-label="Asset source" value={source} onChange={(e) => setSource(e.target.value as typeof source)}><option value="all">All</option><option value="imported">Imported</option><option value="icon-pack">Icon Packs</option><option value="marketplace">Marketplace</option></select></div>
    <div className="asset-import"><input aria-label="Asset path" placeholder="Image path…" value={path} onChange={(e) => setPath(e.target.value)} /><button disabled={!path.trim()} onClick={() => void onImport(path.trim()).then(() => setPath(''))}>Import</button></div>
    <div className="asset-grid">{filtered.map((asset) => <button key={asset.id} className="asset-card" onClick={() => onPick(asset.id, role)} title={asset.path}><span className="asset-thumb">◆</span><strong>{asset.name}</strong><small>{asset.source}</small></button>)}{filtered.length === 0 && <p className="muted">No matching assets.</p>}</div>
  </div>;
}
