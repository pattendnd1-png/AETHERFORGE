import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AppSidebar } from './AppSidebar';

describe('AppSidebar', () => {
  it('renders canonical OpenDeck+ sections with a single selected section', () => {
    const onSectionChange = vi.fn();
    render(<AppSidebar section="buttons" onSectionChange={onSectionChange} />);
    const nav = screen.getByRole('navigation', { name: 'OpenDeck sections' });
    expect(nav).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Buttons' })).toHaveAttribute('aria-current', 'page');
    fireEvent.click(screen.getByRole('button', { name: 'Dials' }));
    expect(onSectionChange).toHaveBeenCalledWith('dials');
    for (const label of ['Touch Strip', 'Profiles', 'Plugins', 'Settings']) {
      expect(screen.getByRole('button', { name: label })).toBeInTheDocument();
    }
  });
});
