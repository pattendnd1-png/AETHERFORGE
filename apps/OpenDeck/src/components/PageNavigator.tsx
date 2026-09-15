import type { Profile } from '../model/workspace';

interface PageNavigatorProps {
  profile: Profile;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}

export function PageNavigator({ profile, onSelect, onAdd, onDuplicate, onDelete }: PageNavigatorProps) {
  const index = Math.max(0, profile.pages.findIndex((page) => page.id === profile.active_page_id));
  const page = profile.pages[index];
  return <div className="page-navigator">
    <button disabled={index <= 0} onClick={() => onSelect(profile.pages[index - 1]?.id ?? page.id)}>‹</button>
    <select aria-label="Page" value={page.id} onChange={(event) => onSelect(event.target.value)}>{profile.pages.map((candidate, i) => <option key={candidate.id} value={candidate.id}>{i + 1}. {candidate.name}</option>)}</select>
    <button disabled={index >= profile.pages.length - 1} onClick={() => onSelect(profile.pages[index + 1]?.id ?? page.id)}>›</button>
    <span className="nav-separator" />
    <button title="Add page" onClick={onAdd}>＋</button>
    <button title="Duplicate page" onClick={onDuplicate}>⧉</button>
    <button title="Delete page" disabled={profile.pages.length <= 1} onClick={onDelete}>−</button>
  </div>;
}
