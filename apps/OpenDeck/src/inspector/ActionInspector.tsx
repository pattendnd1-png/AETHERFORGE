import { getActionDefinition } from '../model/actions';
import type { ControlSlot, Interaction, Page, Profile } from '../model/workspace';

const INTERACTIONS: Record<ControlSlot['kind'], Interaction[]> = {
  key: ['press', 'longPress'],
  dial: ['press', 'rotateLeft', 'rotateRight'],
  touch: ['touch', 'press'],
};

interface Props {
  slot: ControlSlot;
  interaction: Interaction;
  pages: Page[];
  profiles: Profile[];
  onInteraction: (value: Interaction) => void;
  onConfig: (patch: Record<string, unknown>) => void;
  onClear: () => void;
  onTest: () => void;
}

export function ActionInspector({ slot, interaction, pages, profiles, onInteraction, onConfig, onClear, onTest }: Props) {
  const binding = slot.bindings[interaction];
  const definition = binding ? getActionDefinition(binding.definitionId) : null;
  return <div className="inspector-form">
    <label><span>Interaction</span><select value={interaction} onChange={(e) => onInteraction(e.target.value as Interaction)}>{INTERACTIONS[slot.kind].map((value) => <option key={value} value={value}>{value}</option>)}</select></label>
    {!binding || !definition ? <p className="muted">Choose an action from the Action Library.</p> : <>
      <div className="binding-title"><strong>{definition.group}</strong><span>{definition.label}</span></div>
      {definition.inspector.map((field) => {
        const options = field.key === 'pageId'
          ? pages.map((page) => ({ value: page.id, label: page.name }))
          : field.key === 'profileId'
            ? profiles.map((profile) => ({ value: profile.id, label: profile.name }))
            : (field.options ?? []).map((option) => ({ value: option, label: option }));
        return <label key={field.key}><span>{field.label}</span>{field.kind === 'select' ? <select value={String(binding.config[field.key] ?? '')} onChange={(e) => onConfig({ [field.key]: e.target.value })}><option value="">Choose…</option>{options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select> : <input value={String(binding.config[field.key] ?? '')} onChange={(e) => onConfig({ [field.key]: e.target.value })} />}</label>;
      })}
      <div className="inspector-actions"><button onClick={onTest}>Test Action</button><button className="danger subtle" onClick={onClear}>Clear Binding</button></div>
    </>}
  </div>;
}
