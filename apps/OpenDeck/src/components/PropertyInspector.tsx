import { useEffect, useState } from 'react';
import type { Appearance, AppearanceOverride, ControlSlot, Interaction } from '../model/workspace';
import { ActionInspector } from '../inspector/ActionInspector';
import { AppearanceInspector } from '../inspector/AppearanceInspector';
import { StatesInspector } from '../inspector/StatesInspector';

interface Props {
  slot?: ControlSlot;
  onConfig: (interaction: Interaction, patch: Record<string, unknown>) => void;
  onClear: (interaction: Interaction) => void;
  onAppearance: (patch: Partial<Appearance>) => void;
  onState: (patch: AppearanceOverride) => void;
  onResetState: () => void;
  onCopyState: () => void;
  onOpenAssets: (role: 'icon'|'background') => void;
}

export function PropertyInspector({ slot, onConfig, onClear, onAppearance, onState, onResetState, onCopyState, onOpenAssets }: Props) {
  const [tab, setTab] = useState<'action'|'appearance'|'states'>('action');
  const [interaction, setInteraction] = useState<Interaction>('press');
  useEffect(() => { setInteraction(slot?.kind === 'touch' ? 'touch' : 'press'); }, [slot?.id, slot?.kind]);
  return <section className="property-inspector">
    <div className="inspector-header"><div><strong>Property Inspector</strong><span>{slot ? `${slot.kind} ${slot.position + 1}` : 'No selection'}</span></div><nav>{(['action','appearance','states'] as const).map((name) => <button key={name} className={tab === name ? 'active' : ''} onClick={() => setTab(name)}>{name[0].toUpperCase()+name.slice(1)}</button>)}</nav></div>
    <div className="inspector-body">{!slot ? <p className="muted">Select a key, dial, or touch region.</p> : tab === 'action' ? <ActionInspector slot={slot} interaction={interaction} onInteraction={setInteraction} onConfig={(patch) => onConfig(interaction, patch)} onClear={() => onClear(interaction)} /> : tab === 'appearance' ? <AppearanceInspector slot={slot} onUpdate={onAppearance} onOpenAssets={onOpenAssets} /> : <StatesInspector slot={slot} onUpdate={onState} onReset={onResetState} onCopyDefault={onCopyState} />}</div>
  </section>;
}
