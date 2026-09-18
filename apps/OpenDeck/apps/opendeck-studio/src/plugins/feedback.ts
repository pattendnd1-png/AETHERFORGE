import { cloneWorkspace, getActivePage, type ActionInstance, type AssetRecord, type ControlSlot, type Workspace } from '../model/workspace';
import { isPluginDefinitionId } from '../model/plugin-actions';
import type { PluginFeedback } from './types';

function contextOf(binding: ActionInstance | undefined): string | null {
  if (!binding || !isPluginDefinitionId(binding.definitionId)) return null;
  return typeof binding.config.context === 'string' ? binding.config.context : null;
}

function slotContexts(slot: ControlSlot): string[] {
  const contexts = new Set<string>();
  const maps = [slot.bindings, ...(slot.dialStack?.entries.map((entry) => entry.bindings) ?? []), ...(slot.actionWheel?.entries.map((entry) => entry.bindings) ?? [])];
  for (const bindings of maps) {
    for (const binding of Object.values(bindings)) {
      const context = contextOf(binding);
      if (context) contexts.add(context);
    }
  }
  return [...contexts];
}

export interface AppliedPluginFeedback {
  workspace: Workspace;
  previewDataUrls: Record<string, string>;
}

export function applyPluginFeedback(workspace: Workspace, feedbackByContext: Record<string, PluginFeedback>): AppliedPluginFeedback {
  const next = cloneWorkspace(workspace);
  const previews: Record<string, string> = {};
  const assets = new Map(next.assets.map((asset) => [asset.id, asset]));

  for (const profile of next.profiles) {
    for (const page of profile.pages) {
      const slots = [...page.slots.keys, ...page.slots.dials, ...page.slots.touch_regions, page.touchStrip.unifiedSlot];
      for (const slot of slots) {
        const contexts = slotContexts(slot);
        for (const context of contexts) {
          const feedback = feedbackByContext[context];
          if (!feedback) continue;
          if (typeof feedback.title === 'string') slot.appearance.title = feedback.title;
          if (feedback.imageAssetId && feedback.imagePath) {
            const asset: AssetRecord = {
              id: feedback.imageAssetId,
              name: `Plugin feedback ${context}`,
              path: feedback.imagePath,
              source: 'builtin',
              mime: feedback.imageDataUrl?.startsWith('data:image/jpeg') ? 'image/jpeg' : 'image/png',
              sha256: feedback.imageAssetId.replace(/^plugin-feedback-/, ''),
            };
            assets.set(asset.id, asset);
            slot.appearance.iconAssetId = asset.id;
            if (feedback.imageDataUrl) previews[asset.id] = feedback.imageDataUrl;
          }
          if (typeof feedback.state === 'number' && feedback.state > 0 && slot.states.active) {
            slot.appearance = { ...slot.appearance, ...slot.states.active };
          }
        }
      }
    }
  }

  next.assets = [...assets.values()];
  void getActivePage(next);
  return { workspace: next, previewDataUrls: previews };
}
