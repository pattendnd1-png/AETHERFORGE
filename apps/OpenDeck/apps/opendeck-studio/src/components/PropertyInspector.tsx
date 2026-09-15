import { useEffect, useState } from 'react';
import type { Appearance, AppearanceOverride, ControlSlot, Interaction, Page, Profile } from '../model/workspace';
import { ActionInspector } from '../inspector/ActionInspector';
import { AppearanceInspector } from '../inspector/AppearanceInspector';
import { StatesInspector } from '../inspector/StatesInspector';

interface Props {
  slot?: ControlSlot;
  pages: Page[];
  profiles: Profile[];
  collapsed: boolean;
  onToggleCollapsed: () => void;
  onConfig: (interaction: Interaction, patch: Record<string, unknown>) => void;
  onClear: (interaction: Interaction) => void;
  onAppearance: (patch: Partial<Appearance>) => void;
  onState: (patch: AppearanceOverride) => void;
  onResetState: () => void;
  onCopyState: () => void;
  previewState: 'default'|'active';
  onPreviewState: (state: 'default'|'active') => void;
  onOpenAssets: (role: 'icon'|'background', target: 'base'|'active') => void;
}

export function PropertyInspector({ slot, pages, profiles, collapsed, onToggleCollapsed, onConfig, onClear, onAppearance, onState, onResetState, onCopyState, previewState, onPreviewState, onOpenAssets }: Props) {
  const [tab, setTab] = useState<'action'|'appearance'|'states'>('action');
  const [interaction, setInteraction] = useState<Interaction>('press');
  useEffect(() => { setInteraction(slot?.kind === 'touch' ? 'touch' : 'press'); }, [slot?.id, slot?.kind]);

  if (collapsed) {
    return <section className="property-inspector collapsed"><div className="inspector-header collapsed-header"><div><strong>Property Inspector</strong><span>{slot ? `${slot.kind} ${slot.position + 1}` : 'No selection'}</span></div><button aria-label="Expand property inspector" title="Expand Property Inspector" onClick={onToggleCollapsed}>⌃</button></div></section>;
  }

  return <section className="property-inspector">
    <div className="inspector-header"><div><strong>Property Inspector</strong><span>{slot ? `${slot.kind} ${slot.position + 1}` : 'No selection'}</span></div><nav>{(['action','appearance','states'] as const).map((name) => <button key={name} className={tab === name ? 'active' : ''} onClick={() => { setTab(name); if (name !== 'states') onPreviewState('default'); }}>{name[0].toUpperCase()+name.slice(1)}</button>)}<button aria-label="Collapse property inspector" title="Collapse Property Inspector" onClick={onToggleCollapsed}>⌄</button></nav></div>
    <div className="inspector-body">{!slot ? <p className="muted">Select a key, dial, or touch region.</p> : tab === 'action' ? <ActionInspector slot={slot} interaction={interaction} pages={pages} profiles={profiles} onInteraction={setInteraction} onConfig={(patch) => onConfig(interaction, patch)} onClear={() => onClear(interaction)} /> : tab === 'appearance' ? <AppearanceInspector slot={slot} onUpdate={onAppearance} onOpenAssets={(role) => onOpenAssets(role, 'base')} /> : <StatesInspector slot={slot} previewState={previewState} onPreviewState={onPreviewState} onUpdate={onState} onReset={onResetState} onCopyDefault={onCopyState} onOpenAssets={(role) => onOpenAssets(role, 'active')} />}</div>
  </section>;
}
