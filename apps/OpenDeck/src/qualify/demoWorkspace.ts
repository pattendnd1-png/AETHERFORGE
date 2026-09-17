import { createActionInstance } from '../model/actions';
import { createDefaultWorkspace, getActivePage, type Workspace } from '../model/workspace';

const KEY_TITLES = ['Scene', 'Mic', 'Spotify', 'Browser', 'OBS', 'Twitch', 'Discord', 'System'] as const;
const DIAL_TITLES = ['Volume', 'Zoom', 'Brightness', 'Media'] as const;
const KEY_COLORS = ['#10213c', '#33172f', '#123524', '#10233d', '#151921', '#24133a', '#161c46', '#252230'] as const;
const ACCENT_COLORS = ['#1e6fff', '#853cff', '#ffb733', '#ed3fc8'] as const;

export function createQualificationWorkspace(): Workspace {
  const workspace = createDefaultWorkspace();
  const profile = workspace.profiles[0];
  profile.name = 'Default Profile';
  const page = getActivePage(workspace);

  page.slots.keys.forEach((slot, index) => {
    slot.appearance.title = KEY_TITLES[index];
    slot.appearance.fontSize = 13;
    slot.appearance.fontWeight = 600;
    slot.appearance.backgroundColor = KEY_COLORS[index];
  });
  page.slots.keys[0].bindings.press = createActionInstance('obs.scene');
  page.slots.keys[4].bindings.press = createActionInstance('obs.toggleStream');
  page.slots.keys[5].bindings.press = createActionInstance('obs.toggleRecord');
  page.slots.keys[7].bindings.press = createActionInstance('editor.nextPage');

  page.slots.touch_regions.forEach((slot, index) => {
    slot.appearance.title = DIAL_TITLES[index];
    slot.appearance.fontSize = 11;
    slot.appearance.fontWeight = 600;
    slot.appearance.backgroundColor = ACCENT_COLORS[index];
  });

  page.slots.dials.forEach((slot, index) => {
    slot.appearance.title = DIAL_TITLES[index];
    slot.appearance.fontSize = 11;
    slot.appearance.fontWeight = 600;
    slot.appearance.backgroundColor = ACCENT_COLORS[index];
  });
  page.slots.dials[0].bindings.press = createActionInstance('obs.toggleMute');
  page.slots.dials[0].bindings.rotateLeft = createActionInstance('editor.previousPage');
  page.slots.dials[0].bindings.rotateRight = createActionInstance('editor.nextPage');
  page.slots.dials[0].bindings.pressRotateLeft = createActionInstance('editor.previousPage');
  page.slots.dials[0].bindings.pressRotateRight = createActionInstance('editor.nextPage');

  const performanceProfile = structuredClone(profile);
  performanceProfile.id = 'qualification-profile-2';
  performanceProfile.name = 'Performance Profile';
  performanceProfile.active_page_id = performanceProfile.pages[0].id;
  workspace.profiles.push(performanceProfile);

  workspace.preferences.actionPanelWidth = 372;
  workspace.preferences.inspectorHeight = 302;
  workspace.preferences.actionPanelCollapsed = false;
  workspace.preferences.inspectorCollapsed = false;
  return workspace;
}
