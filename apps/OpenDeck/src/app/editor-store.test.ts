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
});
