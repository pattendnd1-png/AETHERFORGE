import { memo, useMemo } from 'react';
import { ACTION_DEFINITIONS, createActionInstance, getActionDefinition } from '../model/actions';
import { externalActionDefinitions, externalActionDescriptor } from '../model/plugin-actions';
import type { ActionInstance } from '../model/workspace';

type Trigger = 'press' | 'doublePress' | 'longPress';

interface Props {
  press: ActionInstance | null;
  doublePress: ActionInstance | null;
  longPress: ActionInstance | null;
  onChange: (trigger: Trigger, action: ActionInstance | null) => void;
}

const LABELS: Record<Trigger, { title: string; detail: string }> = {
  press: { title: 'Press', detail: 'Single press' },
  doublePress: { title: 'Double Press', detail: 'Two presses in quick succession' },
  longPress: { title: 'Press & Hold', detail: 'Hold the key' },
};

export const KeyLogicEditor = memo(function KeyLogicEditor({ press, doublePress, longPress, onChange }: Props) {
  const catalog = useMemo(() => [...ACTION_DEFINITIONS, ...externalActionDefinitions()]
    .filter((definition) => definition.supportedKinds.includes('key'))
    .filter((definition) => definition.id !== 'editor.keyLogic' && definition.id !== 'editor.dialStack' && definition.id !== 'editor.actionWheel')
    .filter((definition) => externalActionDescriptor(definition.id)?.supportedInKeyLogicActions !== false), []);
  const values: Record<Trigger, ActionInstance | null> = { press, doublePress, longPress };
  const describe = (action: ActionInstance | null) => {
    if (!action) return 'Unassigned';
    try { return `${getActionDefinition(action.definitionId).group} — ${getActionDefinition(action.definitionId).label}`; }
    catch { return 'Unavailable plugin action'; }
  };
  return <div className="key-logic-editor">
    <div className="multi-action-heading"><strong>Key Logic</strong><span>Press · Double Press · Press & Hold</span></div>
    {(['press', 'doublePress', 'longPress'] as Trigger[]).map((trigger) => <div className="key-logic-row" key={trigger}>
      <span><strong>{LABELS[trigger].title}</strong><small>{LABELS[trigger].detail}</small></span>
      <select aria-label={`${LABELS[trigger].title} action`} value={values[trigger]?.definitionId ?? ''} onChange={(event) => onChange(trigger, event.target.value ? createActionInstance(event.target.value) : null)}>
        <option value="">Unassigned</option>
        {catalog.map((definition) => <option key={definition.id} value={definition.id}>{definition.group} — {definition.label}</option>)}
      </select>
      <small className="key-logic-current">{describe(values[trigger])}</small>
    </div>)}
  </div>;
});
