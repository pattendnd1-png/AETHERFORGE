import { memo, type DragEvent } from 'react';
import { resolveAppearance, type ControlSelection, type Page } from '../model/workspace';
import { DialControl } from './DialControl';
import { KeyControl } from './KeyControl';
import { TouchControl } from './TouchControl';

const CONTROL_MIME = 'application/x-opendeck-control';
const ACTION_MIME = 'application/x-opendeck-action';

interface DeviceEditorProps {
  page: Page; selection: ControlSelection | null; assetPreviews: Record<string, string>; previewState: 'default'|'active';
  onSelect: (selection: ControlSelection) => void;
  onDropControl: (source: ControlSelection, destination: ControlSelection, copy: boolean) => void;
  onDropAction: (actionId: string, destination: ControlSelection) => void;
}
function parseSelection(raw: string): ControlSelection | null {
  try { const value = JSON.parse(raw) as Partial<ControlSelection>; if ((value.kind === 'key' || value.kind === 'dial' || value.kind === 'touch') && typeof value.slotId === 'string') return { kind: value.kind, slotId: value.slotId }; } catch { /* invalid payload */ }
  return null;
}

export const DeviceEditor = memo(function DeviceEditor({ page, selection, assetPreviews, previewState, onSelect, onDropControl, onDropAction }: DeviceEditorProps) {
  const selected = (kind: ControlSelection['kind'], slotId: string) => selection?.kind === kind && selection.slotId === slotId;
  function dragHandlers(destination: ControlSelection) {
    return {
      onDragStart: (event: DragEvent<HTMLButtonElement>) => { event.dataTransfer.effectAllowed = 'copyMove'; event.dataTransfer.setData(CONTROL_MIME, JSON.stringify(destination)); },
      onDragOver: (event: DragEvent<HTMLButtonElement>) => { if (event.dataTransfer.types.includes(CONTROL_MIME) || event.dataTransfer.types.includes(ACTION_MIME)) { event.preventDefault(); event.dataTransfer.dropEffect = event.ctrlKey || event.altKey ? 'copy' : 'move'; } },
      onDrop: (event: DragEvent<HTMLButtonElement>) => { event.preventDefault(); const actionId = event.dataTransfer.getData(ACTION_MIME); if (actionId) { onDropAction(actionId, destination); return; } const source = parseSelection(event.dataTransfer.getData(CONTROL_MIME)); if (!source || (source.kind === destination.kind && source.slotId === destination.slotId)) return; onDropControl(source, destination, event.ctrlKey || event.altKey); },
    };
  }
  return <section className="device-stage" data-layout-region="device">
    <div className="streamdeck-device" data-testid="deck-plus" data-qualify-element="device">
      <div className="device-brand"><span className="device-brand-mark">▶</span><strong>STREAM DECK+</strong></div>
      <div className="streamdeck-keys">
        {page.slots.keys.map((slot) => { const target = { kind: 'key' as const, slotId: slot.id }; const appearance = resolveAppearance(slot, selected('key', slot.id) ? previewState : 'default'); return <KeyControl key={slot.id} slot={slot} appearance={appearance} selected={selected('key', slot.id)} iconUrl={appearance.iconAssetId ? assetPreviews[appearance.iconAssetId] : undefined} backgroundUrl={appearance.backgroundAssetId ? assetPreviews[appearance.backgroundAssetId] : undefined} onSelect={() => onSelect(target)} {...dragHandlers(target)} />; })}
      </div>
      <div className="streamdeck-touch-display" data-testid="touch-display" data-qualify-element="touch-display">
        {page.slots.touch_regions.map((slot) => { const target = { kind: 'touch' as const, slotId: slot.id }; const appearance = resolveAppearance(slot, selected('touch', slot.id) ? previewState : 'default'); return <TouchControl key={slot.id} slot={slot} appearance={appearance} selected={selected('touch', slot.id)} iconUrl={appearance.iconAssetId ? assetPreviews[appearance.iconAssetId] : undefined} backgroundUrl={appearance.backgroundAssetId ? assetPreviews[appearance.backgroundAssetId] : undefined} onSelect={() => onSelect(target)} {...dragHandlers(target)} />; })}
      </div>
      <div className="streamdeck-dials">
        {page.slots.dials.map((slot) => { const target = { kind: 'dial' as const, slotId: slot.id }; const appearance = resolveAppearance(slot, selected('dial', slot.id) ? previewState : 'default'); return <DialControl key={slot.id} slot={slot} appearance={appearance} selected={selected('dial', slot.id)} iconUrl={appearance.iconAssetId ? assetPreviews[appearance.iconAssetId] : undefined} backgroundUrl={appearance.backgroundAssetId ? assetPreviews[appearance.backgroundAssetId] : undefined} onSelect={() => onSelect(target)} {...dragHandlers(target)} />; })}
      </div>
    </div>
  </section>;
});
