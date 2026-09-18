import assert from 'node:assert/strict';
import {
  replacePluginActionDefinitions,
  parsePluginDefinitionId,
  createPluginActionInstance,
  pluginActionAllowedInMultiAction,
  pluginActionAllowedInKeyLogic,
} from '../apps/opendeck-studio/src/model/plugin-actions.ts';

const plugin = {
  uuid: 'com.test.full-parity', name: 'Parity Test', version: '1.0.0.0', author: 'OpenDeck', description: '',
  sourceKind: 'elgato', root: '/tmp/test', enabled: true, active: true, processState: 'active', compatibility: 'node', runtimeKind: 'node', lastError: null,
  sdkVersion: 3, minimumSoftwareVersion: '7.6', propertyInspectorPath: null, profiles: [],
  actions: [
    { id:'plugin:com.test.full-parity:com.test.action', pluginUuid:'com.test.full-parity', uuid:'com.test.action', name:'Action', category:'Parity Test', controllers:['Keypad','Encoder'], supportedKinds:['key','dial','touch'], defaultInteraction:'press', propertyInspectorPath:null, states:[{name:'Off'},{name:'On'}], supportedInMultiActions:true, supportedInKeyLogicActions:true, disableAutomaticStates:false, visibleInActionsList:true },
    { id:'plugin:com.test.full-parity:hidden', pluginUuid:'com.test.full-parity', uuid:'hidden', name:'Hidden', category:'Parity Test', controllers:['Keypad'], supportedKinds:['key'], defaultInteraction:'press', propertyInspectorPath:null, states:[], supportedInMultiActions:false, supportedInKeyLogicActions:false, disableAutomaticStates:false, visibleInActionsList:false },
  ],
};
const defs = replacePluginActionDefinitions([plugin]);
assert.equal(defs.length,1);
assert.deepEqual(parsePluginDefinitionId(defs[0].id), { pluginUuid:'com.test.full-parity', actionUuid:'com.test.action' });
const instance=createPluginActionInstance(defs[0].id);
assert.equal(instance.definitionId, defs[0].id);
assert.match(String(instance.config.context), /^plugin-context-/);
assert.equal(pluginActionAllowedInMultiAction('plugin:com.test.full-parity:hidden'), false);
assert.equal(pluginActionAllowedInKeyLogic('plugin:com.test.full-parity:hidden'), false);
console.log('OPENDECK_V246_PLUGIN_MODEL=PASS');
