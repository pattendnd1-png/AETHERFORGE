import { describe, expect, it } from 'vitest';
import { actionWheelStatus, activeActionWheelEntry, stepActionWheelIndex } from './action-wheel';
import { createPage } from './workspace';

describe('action wheel model', () => {
  it('resolves the selected wheel action and status', () => {
    const dial = createPage().slots.dials[0];
    dial.actionWheel = {
      behavior: 'rotateSelectPressExecute',
      activeIndex: 1,
      entries: [
        { id: 'one', label: 'OBS', bindings: {} },
        { id: 'two', label: 'Browser', bindings: { press: { definitionId: 'marketplace.open', config: {} } } },
      ],
    };
    expect(activeActionWheelEntry(dial)?.id).toBe('two');
    expect(actionWheelStatus(dial)).toEqual({ label: 'Browser', index: 2, total: 2 });
  });

  it('wraps wheel selection in both directions', () => {
    expect(stepActionWheelIndex(0, 3, -1)).toBe(2);
    expect(stepActionWheelIndex(2, 3, 1)).toBe(0);
  });
});
