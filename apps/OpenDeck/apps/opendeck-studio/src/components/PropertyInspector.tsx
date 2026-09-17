import { memo, useState } from 'react';
import { getActionDefinition } from '../model/actions';
import type { Appearance, AppearanceOverride, ControlSlot, Interaction, Page, Profile, TouchStripConfig, TouchStripMode, TouchStripPresentation, TouchStripRule } from '../model/workspace';
import { ActionInspector } from '../inspector/ActionInspector';
import { AppearanceInspector } from '../inspector/AppearanceInspector';
import { StatesInspector } from '../inspector/StatesInspector';
import { Glyph } from './Glyph';

interface Props {
  slot?: ControlSlot; pages: Page[]; profiles: Profile[]; collapsed: boolean; onToggleCollapsed: () => void;
  onConfig: (interaction: Interaction, patch: Record<string, unknown>) => void; onClear: (interaction: Interaction) => void;
  interaction: Interaction; onInteraction: (value: Interaction) => void; onTest: (interaction: Interaction) => void;
  onAppearance: (patch: Partial<Appearance>) => void; onState: (patch: AppearanceOverride) => void; onResetState: () => void; onCopyState: () => void;
  previewState: 'default'|'active'; onPreviewState: (state: 'default'|'active') => void; onOpenAssets: (role: 'icon'|'background', target: 'base'|'active') => void;
  onSelectPrevious: () => void; onSelectNext: () => void;
  touchStrip: TouchStripConfig; onTouchMode: (mode: TouchStripMode) => void; onTouchFallback: (presentation: TouchStripPresentation) => void; onTouchRules: (rules: TouchStripRule[]) => void;
}
const INTERACTION_LABELS: Partial<Record<Interaction,string>> = { press:'Press', longPress:'Long Press', rotateLeft:'Rotate Left', rotateRight:'Rotate Right', pressRotateLeft:'Press + Rotate Left', pressRotateRight:'Press + Rotate Right', touch:'Touch' };
const INTERACTIONS: Record<ControlSlot['kind'], Interaction[]> = { key:['press','longPress'], dial:['press','rotateLeft','rotateRight','pressRotateLeft','pressRotateRight'], touch:['touch','press'] };
function kindLabel(kind: ControlSlot['kind']) { return kind === 'key' ? 'Button' : kind === 'dial' ? 'Dial' : 'Touch'; }

export const PropertyInspector = memo(function PropertyInspector({ slot, pages, profiles, collapsed, onToggleCollapsed, onConfig, onClear, interaction, onInteraction, onTest, onAppearance, onState, onResetState, onCopyState, previewState, onPreviewState, onOpenAssets, onSelectPrevious, onSelectNext, touchStrip, onTouchMode, onTouchFallback, onTouchRules }: Props) {
  const [tab,setTab]=useState<'action'|'appearance'|'states'>('action');
  const binding=slot?.bindings[interaction]; const actionLabel=binding ? getActionDefinition(binding.definitionId).label : 'Unassigned';
  const selectionLabel=slot ? (slot.id === touchStrip.unifiedSlot.id ? 'Touch Strip' : `${kindLabel(slot.kind)} ${slot.position+1}`) : 'No Selection';
  const rulesText = touchStrip.adaptiveRules.map((rule) => `${rule.pattern}=${rule.presentation}`).join(', ');
  const parseRules = (value: string): TouchStripRule[] => value.split(',').map((part) => { const [rawPattern, rawPresentation] = part.split('=', 2); const pattern = rawPattern?.trim() ?? ''; const presentation = rawPresentation?.trim(); return pattern && (presentation === 'segmented' || presentation === 'unified') ? { pattern, presentation } : null; }).filter((rule): rule is TouchStripRule => rule !== null);
  if (collapsed) return <section className="configuration-strip collapsed" data-layout-region="configuration"><div className="configuration-header"><strong>{slot ? `Configure: ${selectionLabel}` : 'No Selection'}</strong><button aria-label="Expand configuration" onClick={onToggleCollapsed}>⌃</button></div></section>;
  return <section className="configuration-strip" data-layout-region="configuration">
    <div className="configuration-header"><div className="configuration-title"><span className="configuration-dot"/><strong>{slot ? `Configure: ${selectionLabel}` : 'No Selection'}</strong>{slot && <span className="sr-only">{slot.kind} {slot.position + 1}</span>}</div><div className="configuration-selector"><button aria-label="Previous control" onClick={onSelectPrevious} disabled={!slot}>‹</button><span>{slot ? `${selectionLabel} - ${slot.appearance.title || actionLabel}` : 'Select a control'}</span><button aria-label="Next control" onClick={onSelectNext} disabled={!slot}>›</button><button className="config-collapse" aria-label="Collapse configuration" onClick={onToggleCollapsed}>⌄</button></div></div>
    {!slot ? <div className="configuration-empty"><p>Select a key, dial, or touch region to configure it.</p></div> : <div className="configuration-workspace">
      <aside className="interaction-rail" aria-label="Interactions">{INTERACTIONS[slot.kind].map((value)=><button key={value} className={interaction===value?'active':''} aria-pressed={interaction===value} onClick={()=>{onInteraction(value);setTab('action');}}><span className="interaction-icon"><Glyph name={value.includes('rotate')||value.includes('Rotate')?'dial':value==='touch'?'touch':'blank'}/></span><span><strong>{INTERACTION_LABELS[value]}</strong><small>{value==='press'?'Single press action':value==='rotateLeft'?'Counter-clockwise':value==='rotateRight'?'Clockwise':value==='pressRotateLeft'?'Hold and rotate left':value==='pressRotateRight'?'Hold and rotate right':value==='touch'?'Touch interaction':'Hold interaction'}</small></span></button>)}</aside>
      <div className="configuration-content">
        {slot.kind === 'touch' && <div className="touch-strip-mode-editor" aria-label="Touch strip mode">
          <label><span>Touch strip mode</span><select aria-label="Touch strip mode" value={touchStrip.mode} onChange={(event) => onTouchMode(event.target.value as TouchStripMode)}><option value="segmented">Segmented</option><option value="unified">Unified</option><option value="adaptive">Adaptive</option></select></label>
          {touchStrip.mode === 'adaptive' && <><label><span>Fallback</span><select aria-label="Adaptive fallback" value={touchStrip.adaptiveFallback} onChange={(event) => onTouchFallback(event.target.value as TouchStripPresentation)}><option value="segmented">Segmented</option><option value="unified">Unified</option></select></label><label className="touch-rules"><span>Adaptive app rules (app=mode)</span><input aria-label="Adaptive app rules" value={rulesText} onChange={(event) => onTouchRules(parseRules(event.target.value))} placeholder="spotify=unified, obs=segmented" /></label></>}
        </div>}
        <div className="configuration-mode-tabs" role="navigation" aria-label="Configuration sections"><button className={tab==='action'?'active':''} onClick={()=>{setTab('action');onPreviewState('default');}}>Action</button><button className={tab==='appearance'?'active':''} onClick={()=>{setTab('appearance');onPreviewState('default');}}>Appearance</button><button className={tab==='states'?'active':''} onClick={()=>setTab('states')}>States</button></div>
        <div className="assigned-action-heading">Assigned Action</div>
        {tab==='action' ? <ActionInspector slot={slot} interaction={interaction} pages={pages} profiles={profiles} onInteraction={onInteraction} onConfig={(patch)=>onConfig(interaction,patch)} onClear={()=>onClear(interaction)} onTest={()=>onTest(interaction)} /> : tab==='appearance' ? <AppearanceInspector slot={slot} onUpdate={onAppearance} onOpenAssets={(role)=>onOpenAssets(role,'base')} /> : <StatesInspector slot={slot} previewState={previewState} onPreviewState={onPreviewState} onUpdate={onState} onReset={onResetState} onCopyDefault={onCopyState} onOpenAssets={(role)=>onOpenAssets(role,'active')} />}
      </div>
    </div>}
  </section>;
});
