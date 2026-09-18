import { memo, useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { bridge } from '../bridge';
import type { ActionInstance } from '../model/workspace';
import { parsePluginDefinitionId } from '../model/plugin-actions';

interface Props {
  action: ActionInstance;
}

export const PluginPropertyInspector = memo(function PluginPropertyInspector({ action }: Props) {
  const parsed = parsePluginDefinitionId(action.definitionId);
  const pluginUuid = parsed?.pluginUuid ?? '';
  const actionUuid = parsed?.actionUuid ?? '';
  const context = typeof action.config.context === 'string' ? action.config.context : '';
  const [src, setSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setSrc(null);
    setError(null);
    if (!pluginUuid || !actionUuid || !context) {
      setError('Plugin action context is unavailable. Reassign this action to repair it.');
      return () => { alive = false; };
    }
    void bridge.pluginPropertyInspectorSession(pluginUuid, actionUuid, context)
      .then((session) => {
        if (alive) setSrc(convertFileSrc(session.url));
      })
      .catch((reason) => {
        if (alive) setError(String(reason));
      });
    return () => { alive = false; };
  }, [pluginUuid, actionUuid, context]);

  if (error) return <div className="plugin-pi-fallback"><strong>Property Inspector</strong><p>{error}</p></div>;
  if (!src) return <div className="plugin-pi-fallback"><strong>Property Inspector</strong><p>Connecting to plugin…</p></div>;
  return <div className="plugin-pi-host">
    <iframe title="Plugin Property Inspector" src={src} sandbox="allow-scripts allow-forms allow-same-origin" />
  </div>;
});
