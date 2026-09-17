import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { VirtualActionList } from './VirtualActionList';

describe('VirtualActionList', () => {
  it('does not mount all 5,000 actions', () => {
    render(<VirtualActionList items={Array.from({ length: 5000 }, (_, i) => i)} rowHeight={42} viewportHeight={420} renderRow={(i) => <div>{i}</div>} />);
    expect(screen.queryByText('4999')).not.toBeInTheDocument();
    expect(document.querySelectorAll('[data-virtual-row]').length).toBeLessThan(40);
  });
});
