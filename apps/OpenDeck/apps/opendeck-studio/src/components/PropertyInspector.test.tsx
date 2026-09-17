import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createQualificationWorkspace } from '../qualify/demoWorkspace';
import { getActivePage, getActiveProfile } from '../model/workspace';
import { PropertyInspector } from './PropertyInspector';

describe('PropertyInspector v2.0.15', () => {
  it('renders the canonical dial interaction rail', () => {
    const workspace=createQualificationWorkspace(); const page=getActivePage(workspace); const slot=page.slots.dials[0]; const profile=getActiveProfile(workspace);
    render(<PropertyInspector slot={slot} pages={profile.pages} profiles={workspace.profiles} collapsed={false} onToggleCollapsed={vi.fn()} onConfig={vi.fn()} onClear={vi.fn()} interaction="press" onInteraction={vi.fn()} onTest={vi.fn()} onAppearance={vi.fn()} onState={vi.fn()} onResetState={vi.fn()} onCopyState={vi.fn()} previewState="default" onPreviewState={vi.fn()} onOpenAssets={vi.fn()} onSelectPrevious={vi.fn()} onSelectNext={vi.fn()} />);
    expect(screen.getByText('Configure: Dial 1')).toBeInTheDocument();
    for (const label of ['Press Single press action','Rotate Left Counter-clockwise','Rotate Right Clockwise','Press + Rotate Left Hold and rotate left','Press + Rotate Right Hold and rotate right']) expect(screen.getByRole('button',{name:label})).toBeInTheDocument();
    expect(screen.getByText('Assigned Action')).toBeInTheDocument();
  });
});
