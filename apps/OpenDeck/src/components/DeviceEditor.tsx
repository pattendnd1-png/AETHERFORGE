import type { ControlSelection, Page } from '../model/workspace';
import { DialControl } from './DialControl';
import { KeyControl } from './KeyControl';
import { TouchControl } from './TouchControl';

interface DeviceEditorProps {
  page: Page;
  selection: ControlSelection | null;
  onSelect: (selection: ControlSelection) => void;
}

export function DeviceEditor({ page, selection, onSelect }: DeviceEditorProps) {
  const selected = (kind: ControlSelection['kind'], slotId: string) => selection?.kind === kind && selection.slotId === slotId;
  return <section className="device-stage">
    <div className="deck-plus" data-testid="deck-plus">
      <div className="key-grid">
        {page.slots.keys.map((slot) => <KeyControl key={slot.id} slot={slot} selected={selected('key', slot.id)} onSelect={() => onSelect({ kind: 'key', slotId: slot.id })} />)}
      </div>
      <div className="touch-strip">
        {page.slots.touch_regions.map((slot) => <TouchControl key={slot.id} slot={slot} selected={selected('touch', slot.id)} onSelect={() => onSelect({ kind: 'touch', slotId: slot.id })} />)}
      </div>
      <div className="dial-row">
        {page.slots.dials.map((slot) => <DialControl key={slot.id} slot={slot} selected={selected('dial', slot.id)} onSelect={() => onSelect({ kind: 'dial', slotId: slot.id })} />)}
      </div>
    </div>
  </section>;
}
