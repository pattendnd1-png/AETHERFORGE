import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createQualificationWorkspace } from '../qualify/demoWorkspace';
import { getActivePage, getActiveProfile } from '../model/workspace';
import { PropertyInspector } from './PropertyInspector';

function dialContainerCallbacks() {
  return {
    onDialStackAdd: vi.fn(),
    onDialStackSelect: vi.fn(),
    onDialStackRename: vi.fn(),
    onDialStackMove: vi.fn(),
    onDialStackRemove: vi.fn(),
    onDialStackRemoveStack: vi.fn(),
    onActionWheelAdd: vi.fn(),
    onActionWheelSelect: vi.fn(),
    onActionWheelRename: vi.fn(),
    onActionWheelMove: vi.fn(),
    onActionWheelRemove: vi.fn(),
    onActionWheelRemoveWheel: vi.fn(),
  };
}

describe('PropertyInspector v2.0.51', () => {
  it('renders the canonical interaction rail for a non-stacked dial', () => {
    const workspace = createQualificationWorkspace();
    const page = getActivePage(workspace);
    const slot = page.slots.dials[2];
    const profile = getActiveProfile(workspace);

    render(
      <PropertyInspector
        slot={slot}
        pages={profile.pages}
        profiles={workspace.profiles}
        collapsed={false}
        onToggleCollapsed={vi.fn()}
        onConfig={vi.fn()}
        onClear={vi.fn()}
        interaction="press"
        onInteraction={vi.fn()}
        onTest={vi.fn()}
        onAppearance={vi.fn()}
        onState={vi.fn()}
        onResetState={vi.fn()}
        onCopyState={vi.fn()}
        previewState="default"
        onPreviewState={vi.fn()}
        onOpenAssets={vi.fn()}
        onSelectPrevious={vi.fn()}
        onSelectNext={vi.fn()}
        {...dialContainerCallbacks()}
        touchStrip={page.touchStrip}
        onTouchMode={vi.fn()}
        onTouchFallback={vi.fn()}
        onTouchRules={vi.fn()}
      />,
    );

    expect(screen.getByText('Configure: Dial 3')).toBeInTheDocument();
    for (const label of [
      'Press Single press action',
      'Rotate Left Counter-clockwise',
      'Rotate Right Clockwise',
      'Press + Rotate Left Hold and rotate left',
      'Press + Rotate Right Hold and rotate right',
    ]) {
      expect(screen.getByRole('button', { name: label })).toBeInTheDocument();
    }
    expect(screen.getByText('Assigned Action')).toBeInTheDocument();
  });

  it('shows stack cycling separately from the active stack action', () => {
    const workspace = createQualificationWorkspace();
    const page = getActivePage(workspace);
    const slot = page.slots.dials[0];
    const profile = getActiveProfile(workspace);

    render(
      <PropertyInspector
        slot={slot}
        pages={profile.pages}
        profiles={workspace.profiles}
        collapsed={false}
        onToggleCollapsed={vi.fn()}
        onConfig={vi.fn()}
        onClear={vi.fn()}
        interaction="press"
        onInteraction={vi.fn()}
        onTest={vi.fn()}
        onAppearance={vi.fn()}
        onState={vi.fn()}
        onResetState={vi.fn()}
        onCopyState={vi.fn()}
        previewState="default"
        onPreviewState={vi.fn()}
        onOpenAssets={vi.fn()}
        onSelectPrevious={vi.fn()}
        onSelectNext={vi.fn()}
        {...dialContainerCallbacks()}
        touchStrip={page.touchStrip}
        onTouchMode={vi.fn()}
        onTouchFallback={vi.fn()}
        onTouchRules={vi.fn()}
      />,
    );

    expect(screen.getByText('Cycles stack entry')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Press Single press action' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('Dial Stack editor')).toBeInTheDocument();
    expect(screen.getByText('Active Stack Action')).toBeInTheDocument();
  });

  it('configures adaptive touch strip mode, fallback, and app rules', () => {
    const workspace = createQualificationWorkspace();
    const page = getActivePage(workspace);
    const profile = getActiveProfile(workspace);
    const slot = page.touchStrip.unifiedSlot;
    page.touchStrip.mode = 'adaptive';
    const onTouchMode = vi.fn();
    const onTouchFallback = vi.fn();
    const onTouchRules = vi.fn();

    render(
      <PropertyInspector
        slot={slot}
        pages={profile.pages}
        profiles={workspace.profiles}
        collapsed={false}
        onToggleCollapsed={vi.fn()}
        onConfig={vi.fn()}
        onClear={vi.fn()}
        interaction="touch"
        onInteraction={vi.fn()}
        onTest={vi.fn()}
        onAppearance={vi.fn()}
        onState={vi.fn()}
        onResetState={vi.fn()}
        onCopyState={vi.fn()}
        previewState="default"
        onPreviewState={vi.fn()}
        onOpenAssets={vi.fn()}
        onSelectPrevious={vi.fn()}
        onSelectNext={vi.fn()}
        {...dialContainerCallbacks()}
        touchStrip={page.touchStrip}
        onTouchMode={onTouchMode}
        onTouchFallback={onTouchFallback}
        onTouchRules={onTouchRules}
      />,
    );

    expect(screen.getByText('Configure: Touch Strip')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Touch strip mode'), { target: { value: 'unified' } });
    fireEvent.change(screen.getByLabelText('Adaptive fallback'), { target: { value: 'unified' } });
    fireEvent.change(screen.getByLabelText('Adaptive app rules'), { target: { value: 'spotify=unified, obs=segmented' } });

    expect(onTouchMode).toHaveBeenCalledWith('unified');
    expect(onTouchFallback).toHaveBeenCalledWith('unified');
    expect(onTouchRules).toHaveBeenCalledWith([
      { pattern: 'spotify', presentation: 'unified' },
      { pattern: 'obs', presentation: 'segmented' },
    ]);
  });
  it('shows action wheel selection separately from its selected action', () => {
    const workspace = createQualificationWorkspace();
    const page = getActivePage(workspace);
    const slot = page.slots.dials[1];
    const profile = getActiveProfile(workspace);

    render(
      <PropertyInspector
        slot={slot}
        pages={profile.pages}
        profiles={workspace.profiles}
        collapsed={false}
        onToggleCollapsed={vi.fn()}
        onConfig={vi.fn()}
        onClear={vi.fn()}
        interaction="rotateRight"
        onInteraction={vi.fn()}
        onTest={vi.fn()}
        onAppearance={vi.fn()}
        onState={vi.fn()}
        onResetState={vi.fn()}
        onCopyState={vi.fn()}
        previewState="default"
        onPreviewState={vi.fn()}
        onOpenAssets={vi.fn()}
        onSelectPrevious={vi.fn()}
        onSelectNext={vi.fn()}
        {...dialContainerCallbacks()}
        touchStrip={page.touchStrip}
        onTouchMode={vi.fn()}
        onTouchFallback={vi.fn()}
        onTouchRules={vi.fn()}
      />,
    );

    expect(screen.getByLabelText('Action Wheel editor')).toBeInTheDocument();
    expect(screen.getByText('Selects wheel action')).toBeInTheDocument();
    expect(screen.getByText('Executes selected action')).toBeInTheDocument();
    expect(screen.getByText('Selected Wheel Action')).toBeInTheDocument();
  });

});
