import { useState } from 'react';
import { getActionDefinition } from '../model/actions';
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
  interaction: Interaction;
  onInteraction: (value: Interaction) => void;
  onTest: (interaction: Interaction) => void;
  onAppearance: (patch: Partial<Appearance>) => void;
  onState: (patch: AppearanceOverride) => void;
  onResetState: () => void;
  onCopyState: () => void;
  previewState: 'default'|'active';
  onPreviewState: (state: 'default'|'active') => void;
  onOpenAssets: (role: 'icon'|'background', target: 'base'|'active') => void;
}

export function PropertyInspector({ slot, pages, profiles, collapsed, onToggleCollapsed, onConfig, onClear, interaction, onInteraction, onTest, onAppearance, onState, onResetState, onCopyState, previewState, onPreviewState, onOpenAssets }: Props) {
  const [tab, setTab] = useState<'action'|'appearance'|'states'>('action');

  const binding = slot?.bindings[interaction];
  const actionLabel = binding ? getActionDefinition(binding.definitionId).label : 'Unassigned';
  const selectionLabel = slot ? `${slot.kind} ${slot.position + 1}` : 'No selection';

  if (collapsed) {
    return <section className="configuration-strip collapsed" data-layout-region="configuration">
      <div className="configuration-header collapsed-header">
        <div className="configuration-identity"><strong>{selectionLabel}</strong>{slot && <span>{actionLabel}</span>}</div>
        <button aria-label="Expand configuration" title="Expand configuration" onClick={onToggleCollapsed}>⌃</button>
      </div>
    </section>;
  }

  return <section className="configuration-strip" data-layout-region="configuration">
    <div className="configuration-header">
      <div className="configuration-identity"><strong>{selectionLabel}</strong>{slot && <span>{actionLabel}</span>}</div>
      <nav aria-label="Configuration sections">
        {slot && (['action','appearance','states'] as const).map((name) => <button key={name} className={tab === name ? 'active' : ''} onClick={() => { setTab(name); if (name !== 'states') onPreviewState('default'); }}>{name[0].toUpperCase()+name.slice(1)}</button>)}
        <button aria-label="Collapse configuration" title="Collapse configuration" onClick={onToggleCollapsed}>⌄</button>
      </nav>
    </div>
    <div className="configuration-body">{!slot ? <p className="muted">Select a key, dial, or touch region to configure it.</p> : tab === 'action' ? <ActionInspector slot={slot} interaction={interaction} pages={pages} profiles={profiles} onInteraction={onInteraction} onConfig={(patch) => onConfig(interaction, patch)} onClear={() => onClear(interaction)} onTest={() => onTest(interaction)} /> : tab === 'appearance' ? <AppearanceInspector slot={slot} onUpdate={onAppearance} onOpenAssets={(role) => onOpenAssets(role, 'base')} /> : <StatesInspector slot={slot} previewState={previewState} onPreviewState={onPreviewState} onUpdate={onState} onReset={onResetState} onCopyDefault={onCopyState} onOpenAssets={(role) => onOpenAssets(role, 'active')} />}</div>
  </section>;
}
