import { describe, expect, it } from 'vitest';
import { createActionInstance } from '../model/actions';
import { createDefaultWorkspace, getActivePage } from '../model/workspace';
import { createEditorState, editorReducer, selectedSlot } from './editor-store';

describe('editor reducer', () => {
  it('keeps dial rotate-left, rotate-right and press independent', () => {
    let state = createEditorState(createDefaultWorkspace());
    const dial = getActivePage(state.workspace).slots.dials[0];
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection: { kind: 'dial', slotId: dial.id } });
    state = editorReducer(state, { type: 'ASSIGN_ACTION', interaction: 'rotateLeft', action: createActionInstance('editor.previousPage') });
    state = editorReducer(state, { type: 'ASSIGN_ACTION', interaction: 'rotateRight', action: createActionInstance('editor.nextPage') });
    expect(selectedSlot(state)?.bindings.rotateLeft?.definitionId).toBe('editor.previousPage');
    expect(selectedSlot(state)?.bindings.rotateRight?.definitionId).toBe('editor.nextPage');
    expect(selectedSlot(state)?.bindings.press).toBeUndefined();
  });

  it('supports undo and redo for appearance changes', () => {
    let state = createEditorState(createDefaultWorkspace());
    const key = getActivePage(state.workspace).slots.keys[0];
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection: { kind: 'key', slotId: key.id } });
    state = editorReducer(state, { type: 'UPDATE_APPEARANCE', patch: { title: 'Live' } });
    expect(selectedSlot(state)?.appearance.title).toBe('Live');
    state = editorReducer(state, { type: 'UNDO' });
    expect(selectedSlot(state)?.appearance.title).toBe('');
    state = editorReducer(state, { type: 'REDO' });
    expect(selectedSlot(state)?.appearance.title).toBe('Live');
  });

  it('never deletes the last page', () => {
    let state = createEditorState(createDefaultWorkspace());
    state = editorReducer(state, { type: 'DELETE_PAGE' });
    expect(getActivePage(state.workspace)).toBeDefined();
  });

  it('moves and copies complete control customization', () => {
    let state = createEditorState(createDefaultWorkspace());
    const page = getActivePage(state.workspace);
    const source = page.slots.keys[0];
    const destination = page.slots.keys[1];
    const copyDestination = page.slots.keys[2];

    state = editorReducer(state, { type: 'SELECT_CONTROL', selection: { kind: 'key', slotId: source.id } });
    state = editorReducer(state, { type: 'ASSIGN_ACTION', interaction: 'press', action: createActionInstance('obs.toggleRecord') });
    state = editorReducer(state, { type: 'UPDATE_APPEARANCE', patch: { title: 'Record', backgroundColor: '#441111' } });

    state = editorReducer(state, {
      type: 'COPY_CONTROL',
      source: { kind: 'key', slotId: source.id },
      destination: { kind: 'key', slotId: copyDestination.id },
    });
    const copiedPage = getActivePage(state.workspace);
    expect(copiedPage.slots.keys[0].appearance.title).toBe('Record');
    expect(copiedPage.slots.keys[2].appearance.title).toBe('Record');
    expect(copiedPage.slots.keys[2].bindings.press?.definitionId).toBe('obs.toggleRecord');

    state = editorReducer(state, {
      type: 'MOVE_CONTROL',
      source: { kind: 'key', slotId: source.id },
      destination: { kind: 'key', slotId: destination.id },
    });
    const movedPage = getActivePage(state.workspace);
    expect(movedPage.slots.keys[1].appearance.title).toBe('Record');
    expect(movedPage.slots.keys[0].appearance.title).toBe('');
  });

  it('updates editor panel preferences without polluting undo history', () => {
    let state = createEditorState(createDefaultWorkspace());
    state = editorReducer(state, { type: 'UPDATE_PREFERENCES', patch: { actionPanelCollapsed: true, actionPanelWidth: 320 } });
    expect(state.workspace.preferences.actionPanelCollapsed).toBe(true);
    expect(state.workspace.preferences.actionPanelWidth).toBe(320);
    expect(state.past).toHaveLength(0);
    expect(state.future).toHaveLength(0);
  });

  it('renames the active page', () => {
    let state = createEditorState(createDefaultWorkspace());
    const page = getActivePage(state.workspace);
    state = editorReducer(state, { type: 'RENAME_PAGE', pageId: page.id, name: 'Streaming' });
    expect(getActivePage(state.workspace).name).toBe('Streaming');
  });

  it('remaps folder targets and active page when duplicating a profile', () => {
    let state = createEditorState(createDefaultWorkspace());
    state = editorReducer(state, { type: 'ADD_PAGE', name: 'Folder Target' });
    const sourceProfile = state.workspace.profiles[0];
    const targetPage = getActivePage(state.workspace);
    const sourcePage = sourceProfile.pages[0];
    state = editorReducer(state, { type: 'SET_ACTIVE_PAGE', pageId: sourcePage.id });
    const key = getActivePage(state.workspace).slots.keys[0];
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection: { kind: 'key', slotId: key.id } });
    state = editorReducer(state, { type: 'ASSIGN_ACTION', interaction: 'press', action: { definitionId: 'editor.folder', config: { pageId: targetPage.id } } });
    state = editorReducer(state, { type: 'UPDATE_ACTION_CONFIG', interaction: 'press', patch: { pageId: targetPage.id } });
    state = editorReducer(state, { type: 'SET_ACTIVE_PAGE', pageId: targetPage.id });
    const activeSourcePageId = getActivePage(state.workspace).id;

    state = editorReducer(state, { type: 'DUPLICATE_PROFILE', profileId: sourceProfile.id });
    const duplicate = state.workspace.profiles.find((profile) => profile.id === state.workspace.active_profile_id)!;
    expect(duplicate.active_page_id).not.toBe(activeSourcePageId);
    expect(duplicate.pages.find((page) => page.id === duplicate.active_page_id)?.name).toBe('Folder Target');
    const duplicateFolder = duplicate.pages[0].slots.keys[0];
    expect(duplicateFolder.folderTarget).toBe(duplicate.pages[1].id);
    expect(duplicateFolder.bindings.press?.config.pageId).toBe(duplicate.pages[1].id);
  });

  it('clears folder references when a target page is deleted', () => {
    let state = createEditorState(createDefaultWorkspace());
    const sourcePageId = getActivePage(state.workspace).id;
    state = editorReducer(state, { type: 'ADD_PAGE', name: 'Target' });
    const targetPageId = getActivePage(state.workspace).id;
    state = editorReducer(state, { type: 'SET_ACTIVE_PAGE', pageId: sourcePageId });
    const key = getActivePage(state.workspace).slots.keys[0];
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection: { kind: 'key', slotId: key.id } });
    state = editorReducer(state, { type: 'ASSIGN_ACTION', interaction: 'press', action: { definitionId: 'editor.folder', config: { pageId: targetPageId } } });
    state = editorReducer(state, { type: 'UPDATE_ACTION_CONFIG', interaction: 'press', patch: { pageId: targetPageId } });
    state = editorReducer(state, { type: 'SET_ACTIVE_PAGE', pageId: targetPageId });
    state = editorReducer(state, { type: 'DELETE_PAGE' });
    const remainingKey = getActivePage(state.workspace).slots.keys[0];
    expect(remainingKey.folderTarget).toBeNull();
    expect(remainingKey.bindings.press?.config.pageId).toBe('');
  });

  it('clears switch-profile references when a target profile is deleted', () => {
    let state = createEditorState(createDefaultWorkspace());
    const sourceProfileId = state.workspace.active_profile_id;
    state = editorReducer(state, { type: 'CREATE_PROFILE', name: 'Target Profile' });
    const targetProfileId = state.workspace.active_profile_id;
    state = editorReducer(state, { type: 'SET_ACTIVE_PROFILE', profileId: sourceProfileId });
    const key = getActivePage(state.workspace).slots.keys[0];
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection: { kind: 'key', slotId: key.id } });
    state = editorReducer(state, { type: 'ASSIGN_ACTION', interaction: 'press', action: { definitionId: 'editor.switchProfile', config: { profileId: targetProfileId } } });
    state = editorReducer(state, { type: 'DELETE_PROFILE', profileId: targetProfileId });
    expect(getActivePage(state.workspace).slots.keys[0].bindings.press?.config.profileId).toBe('');
  });


  it('creates a dial stack from existing bindings and assigns into the active entry', () => {
    let state = createEditorState(createDefaultWorkspace());
    const dial = getActivePage(state.workspace).slots.dials[0];
    const selection = { kind: 'dial' as const, slotId: dial.id };
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection });
    state = editorReducer(state, { type: 'ASSIGN_ACTION', interaction: 'rotateLeft', action: createActionInstance('editor.previousPage') });
    state = editorReducer(state, { type: 'CREATE_DIAL_STACK', selection });
    expect(selectedSlot(state)?.dialStack?.entries).toHaveLength(1);
    expect(selectedSlot(state)?.dialStack?.entries[0].bindings.rotateLeft?.definitionId).toBe('editor.previousPage');
    state = editorReducer(state, { type: 'ASSIGN_ACTION_TO_CONTROL', selection, interaction: 'rotateRight', action: createActionInstance('editor.nextPage'), title: 'Next Page' });
    expect(selectedSlot(state)?.dialStack?.entries[0].bindings.rotateRight?.definitionId).toBe('editor.nextPage');
  });

  it('adds, selects, reorders, cycles, and removes dial stack entries', () => {
    let state = createEditorState(createDefaultWorkspace());
    const dial = getActivePage(state.workspace).slots.dials[0];
    const selection = { kind: 'dial' as const, slotId: dial.id };
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection });
    state = editorReducer(state, { type: 'CREATE_DIAL_STACK', selection });
    state = editorReducer(state, { type: 'RENAME_DIAL_STACK_ENTRY', entryId: selectedSlot(state)!.dialStack!.entries[0].id, label: 'Volume' });
    state = editorReducer(state, { type: 'ADD_DIAL_STACK_ENTRY', label: 'OBS' });
    const stack = selectedSlot(state)!.dialStack!;
    expect(stack.activeIndex).toBe(1);
    expect(stack.entries.map((entry) => entry.label)).toEqual(['Volume', 'OBS']);
    const obsId = stack.entries[1].id;
    state = editorReducer(state, { type: 'MOVE_DIAL_STACK_ENTRY', entryId: obsId, direction: -1 });
    expect(selectedSlot(state)!.dialStack!.entries.map((entry) => entry.label)).toEqual(['OBS', 'Volume']);
    expect(selectedSlot(state)!.dialStack!.activeIndex).toBe(0);
    state = editorReducer(state, { type: 'CYCLE_DIAL_STACK', selection });
    expect(selectedSlot(state)!.dialStack!.activeIndex).toBe(1);
    state = editorReducer(state, { type: 'REMOVE_DIAL_STACK_ENTRY', entryId: obsId });
    expect(selectedSlot(state)!.dialStack!.entries).toHaveLength(1);
    state = editorReducer(state, { type: 'REMOVE_DIAL_STACK' });
    expect(selectedSlot(state)!.dialStack).toBeNull();
  });

  it('copies dial stacks with complete control customization', () => {
    let state = createEditorState(createDefaultWorkspace());
    const page = getActivePage(state.workspace);
    const source = { kind: 'dial' as const, slotId: page.slots.dials[0].id };
    const destination = { kind: 'dial' as const, slotId: page.slots.dials[1].id };
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection: source });
    state = editorReducer(state, { type: 'CREATE_DIAL_STACK', selection: source });
    state = editorReducer(state, { type: 'ADD_DIAL_STACK_ENTRY', label: 'Second' });
    const sourceIds = selectedSlot(state)!.dialStack!.entries.map((entry) => entry.id);
    state = editorReducer(state, { type: 'COPY_CONTROL', source, destination });
    const copiedStack = getActivePage(state.workspace).slots.dials[1].dialStack!;
    expect(copiedStack.entries).toHaveLength(2);
    expect(copiedStack.entries.map((entry) => entry.id)).not.toEqual(sourceIds);
  });

  it('regenerates dial stack entry ids when duplicating a profile', () => {
    let state = createEditorState(createDefaultWorkspace());
    const profileId = state.workspace.active_profile_id;
    const dial = getActivePage(state.workspace).slots.dials[0];
    const selection = { kind: 'dial' as const, slotId: dial.id };
    state = editorReducer(state, { type: 'SELECT_CONTROL', selection });
    state = editorReducer(state, { type: 'CREATE_DIAL_STACK', selection });
    state = editorReducer(state, { type: 'ADD_DIAL_STACK_ENTRY', label: 'Second' });
    const originalIds = selectedSlot(state)!.dialStack!.entries.map((entry) => entry.id);
    state = editorReducer(state, { type: 'DUPLICATE_PROFILE', profileId });
    const duplicate = state.workspace.profiles.find((profile) => profile.id === state.workspace.active_profile_id)!;
    const duplicateIds = duplicate.pages[0].slots.dials[0].dialStack!.entries.map((entry) => entry.id);
    expect(duplicateIds).toHaveLength(2);
    expect(duplicateIds).not.toEqual(originalIds);
    expect(new Set([...originalIds, ...duplicateIds]).size).toBe(4);
  });

});
