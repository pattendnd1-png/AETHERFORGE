import { memo, useMemo, useState } from 'react';
import { ACTION_DEFINITIONS, createActionInstance, getActionDefinition } from '../model/actions';
import { externalActionDefinitions, pluginActionAllowedInMultiAction } from '../model/plugin-actions';
import type { ActionInstance, ControlKind } from '../model/workspace';

interface Props {
  steps: ActionInstance[];
  controlKind: ControlKind;
  onChange: (steps: ActionInstance[]) => void;
}

export const MultiActionEditor = memo(function MultiActionEditor({ steps, controlKind, onChange }: Props) {
  const catalog = useMemo(() => [...ACTION_DEFINITIONS, ...externalActionDefinitions()]
    .filter((definition) => definition.id !== 'editor.multiAction' && definition.id !== 'editor.keyLogic' && definition.id !== 'editor.dialStack' && definition.id !== 'editor.actionWheel' && definition.supportedKinds.includes(controlKind))
    .filter((definition) => pluginActionAllowedInMultiAction(definition.id)), [controlKind]);
  const [choice, setChoice] = useState(catalog[0]?.id ?? '');

  const move = (index: number, delta: -1 | 1) => {
    const target = index + delta;
    if (target < 0 || target >= steps.length) return;
    const next = structuredClone(steps);
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };
  const add = () => {
    if (!choice) return;
    onChange([...steps, createActionInstance(choice)]);
  };
  return <div className="multi-action-editor">
    <div className="multi-action-heading"><strong>Multi Action</strong><span>{steps.length} step{steps.length === 1 ? '' : 's'} · executed in order</span></div>
    <div className="multi-action-steps">
      {steps.length === 0 && <p className="muted">Add actions below. Plugin steps are dispatched with <code>isInMultiAction=true</code>.</p>}
      {steps.map((step, index) => {
        let label = step.definitionId;
        let group = 'Action';
        try { const definition = getActionDefinition(step.definitionId); label = definition.label; group = definition.group; } catch { /* unavailable plugin remains visible */ }
        return <div className="multi-action-step" key={`${step.definitionId}-${index}`}>
          <span><em>{index + 1}</em><span><strong>{label}</strong><small>{group}</small></span></span>
          <span className="multi-action-controls"><button onClick={() => move(index, -1)} disabled={index === 0} aria-label="Move step up">↑</button><button onClick={() => move(index, 1)} disabled={index === steps.length - 1} aria-label="Move step down">↓</button><button onClick={() => onChange(steps.filter((_, candidate) => candidate !== index))} aria-label="Remove step">×</button></span>
        </div>;
      })}
    </div>
    <div className="multi-action-add"><select aria-label="Multi Action step" value={choice} onChange={(event) => setChoice(event.target.value)}>{catalog.map((definition) => <option key={definition.id} value={definition.id}>{definition.group} — {definition.label}</option>)}</select><button onClick={add} disabled={!choice}>Add Step</button></div>
  </div>;
});
