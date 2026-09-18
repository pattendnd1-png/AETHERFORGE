import assert from 'node:assert/strict';
import {
  activeActionWheelEntry,
  actionWheelStatus,
  stepActionWheelIndex,
} from '../apps/opendeck-studio/src/model/action-wheel.ts';

const slot = {
  kind: 'dial',
  actionWheel: {
    behavior: 'rotateSelectPressExecute',
    activeIndex: 0,
    entries: [
      { id: 'one', label: 'OBS', bindings: { press: { definitionId: 'obs.toggleStream', config: {} } } },
      { id: 'two', label: 'Spotify', bindings: { press: { definitionId: 'editor.nextPage', config: {} } } },
      { id: 'three', label: 'Browser', bindings: { press: { definitionId: 'marketplace.open', config: {} } } },
    ],
  },
};
assert.equal(activeActionWheelEntry(slot)?.id, 'one');
assert.deepEqual(actionWheelStatus(slot), { label: 'OBS', index: 1, total: 3 });
assert.equal(stepActionWheelIndex(0, 3, 1), 1);
assert.equal(stepActionWheelIndex(0, 3, -1), 2);
assert.equal(stepActionWheelIndex(2, 3, 1), 0);
assert.equal(stepActionWheelIndex(99, 3, 1), 0);
assert.equal(stepActionWheelIndex(0, 0, 1), 0);
console.log('OPENDECK_V232_ACTION_WHEEL_MODEL=PASS');
