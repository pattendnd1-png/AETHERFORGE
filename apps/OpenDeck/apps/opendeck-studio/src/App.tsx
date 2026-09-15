import { useEffect, useMemo, useState } from 'react';
import { bridge } from './bridge';
import type { DeckAction, MarketplaceItem, TwitchDeviceCode, TwitchIdentity } from './types';

const EMPTY_KEYS = Array.from({ length: 8 }, () => null as DeckAction | null);
const KEY_STORAGE = 'opendeck-v2.keys';

function loadKeys(): Array<DeckAction | null> {
  try {
    if (typeof window === 'undefined') return [...EMPTY_KEYS];
    const saved = window.localStorage.getItem(KEY_STORAGE);
    if (!saved) return [...EMPTY_KEYS];
    const parsed = JSON.parse(saved);
    return Array.isArray(parsed) && parsed.length === 8 ? parsed as Array<DeckAction | null> : [...EMPTY_KEYS];
  } catch {
    return [...EMPTY_KEYS];
  }
}

function persistKeys(keys: Array<DeckAction | null>): void {
  try {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(KEY_STORAGE, JSON.stringify(keys));
  } catch {
    // Storage may be unavailable in hardened webviews or test environments.
    // Editing must remain usable even when persistence is temporarily unavailable.
  }
}
const ACTION_LIBRARY: Array<{ id: string; label: string; action: DeckAction }> = [
  { id: 'obs.scene', label: 'OBS · Scene', action: { kind: 'obs.scene', label: 'Scene', sceneName: '' } },
  { id: 'obs.toggleStream', label: 'OBS · Toggle Stream', action: { kind: 'obs.toggleStream', label: 'Toggle Stream' } },
  { id: 'obs.toggleRecord', label: 'OBS · Toggle Record', action: { kind: 'obs.toggleRecord', label: 'Toggle Record' } },
  { id: 'obs.toggleMute', label: 'OBS · Toggle Input Mute', action: { kind: 'obs.toggleMute', label: 'Toggle Mute', inputName: '' } },
  { id: 'marketplace.open', label: 'Elgato · Open Marketplace', action: { kind: 'marketplace.open', label: 'Marketplace' } },
];

function cloneAction(action: DeckAction): DeckAction {
  return JSON.parse(JSON.stringify(action)) as DeckAction;
}

export default function App() {
  const [profile, setProfile] = useState('Default Profile');
  const [keys, setKeys] = useState<Array<DeckAction | null>>(loadKeys);
  const [selectedKey, setSelectedKey] = useState(0);
  const [pendingAction, setPendingAction] = useState<DeckAction | null>(null);
  const [panel, setPanel] = useState<'none' | 'connections' | 'marketplace'>('none');
  const [obsHost, setObsHost] = useState('127.0.0.1');
  const [obsPort, setObsPort] = useState(4455);
  const [obsPassword, setObsPassword] = useState('');
  const [obsStatus, setObsStatus] = useState('Not connected');
  const [scenes, setScenes] = useState<string[]>([]);
  const [twitchIdentity, setTwitchIdentity] = useState<TwitchIdentity | null>(null);
  const [twitchCode, setTwitchCode] = useState<TwitchDeviceCode | null>(null);
  const [twitchMessage, setTwitchMessage] = useState('Signed out');
  const [marketItems, setMarketItems] = useState<MarketplaceItem[]>([]);
  const [message, setMessage] = useState('Ready');

  useEffect(() => {
    persistKeys(keys);
  }, [keys]);

  useEffect(() => {
    void bridge.twitchStatus().then((identity) => {
      setTwitchIdentity(identity);
      if (identity) setTwitchMessage(`Signed in as ${identity.login}`);
    }).catch(() => undefined);
  }, []);

  useEffect(() => {
    if (!twitchCode || twitchIdentity) return;
    const timer = window.setInterval(() => {
      void bridge.twitchPollAuth(twitchCode.device_code).then((identity) => {
        if (identity) {
          setTwitchIdentity(identity);
          setTwitchMessage(`Signed in as ${identity.login}`);
          setTwitchCode(null);
        }
      }).catch((error) => setTwitchMessage(String(error)));
    }, Math.max(2, twitchCode.interval) * 1000);
    return () => window.clearInterval(timer);
  }, [twitchCode, twitchIdentity]);

  const selectedAction = keys[selectedKey];
  const actionGroups = useMemo(() => ({
    OBS: ACTION_LIBRARY.filter((item) => item.id.startsWith('obs.')),
    Elgato: ACTION_LIBRARY.filter((item) => item.id.startsWith('marketplace.')),
  }), []);

  function chooseKey(index: number) {
    setSelectedKey(index);
    if (pendingAction) {
      setKeys((current) => current.map((value, i) => i === index ? cloneAction(pendingAction) : value));
      setPendingAction(null);
      setMessage(`Assigned ${pendingAction.label} to key ${index + 1}`);
    }
  }

  async function execute(action: DeckAction | null) {
    if (!action) return;
    try {
      if (action.kind === 'obs.scene') {
        if (!action.sceneName) throw new Error('Choose a scene in Property Inspector first.');
        await bridge.obsSetScene(action.sceneName);
      } else if (action.kind === 'obs.toggleStream') {
        await bridge.obsToggleStream();
      } else if (action.kind === 'obs.toggleRecord') {
        await bridge.obsToggleRecord();
      } else if (action.kind === 'obs.toggleMute') {
        if (!action.inputName) throw new Error('Enter an OBS input name in Property Inspector first.');
        await bridge.obsToggleMute(action.inputName);
      } else if (action.kind === 'marketplace.open') {
        await bridge.openExternal('https://marketplace.elgato.com');
      }
      setMessage(`${action.label} executed`);
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function connectObs() {
    setObsStatus('Connecting…');
    try {
      await bridge.obsSaveConfig(obsHost, obsPort, obsPassword);
      const status = await bridge.obsStatus();
      setObsStatus(status);
      const names = await bridge.obsScenes();
      setScenes(names);
    } catch (error) {
      setObsStatus(`Error: ${String(error)}`);
    }
  }

  async function beginTwitch() {
    setTwitchMessage('Starting Twitch sign-in…');
    try {
      const code = await bridge.twitchBeginAuth();
      setTwitchCode(code);
      setTwitchMessage(`Enter code ${code.user_code} in Twitch`);
      await bridge.openExternal(code.verification_uri);
    } catch (error) {
      setTwitchMessage(`Error: ${String(error)}`);
    }
  }

  async function scanMarketplace() {
    try {
      const items = await bridge.scanMarketplace();
      setMarketItems(items);
    } catch (error) {
      setMessage(String(error));
    }
  }

  function updateSelected(patch: object) {
    setKeys((current) => current.map((value, i) => i === selectedKey && value ? ({ ...value, ...patch } as DeckAction) : value));
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="selectors">
          <label><span>Device</span><select aria-label="Device" value="Stream Deck +" onChange={() => undefined}><option>Stream Deck +</option></select></label>
          <label><span>Profile</span><select aria-label="Profile" value={profile} onChange={(e) => setProfile(e.target.value)}><option>Default Profile</option><option>Streaming</option></select></label>
        </div>
        <div className="top-actions">
          <button onClick={() => setPanel(panel === 'connections' ? 'none' : 'connections')}>Accounts / Connections</button>
          <button onClick={() => { setPanel('marketplace'); void scanMarketplace(); }}>Marketplace</button>
        </div>
      </header>

      <main className="workspace">
        <section className="editor-column">
          <div className="device-stage">
            <div className="deck-plus" data-testid="deck-plus">
              <div className="key-grid">
                {keys.map((action, index) => (
                  <button key={index} data-testid="deck-key" className={selectedKey === index ? 'deck-key selected' : 'deck-key'}
                    onClick={() => chooseKey(index)} onDoubleClick={() => void execute(action)} title="Click to select; double-click to execute">
                    <span>{action?.label ?? '+'}</span><small>{index + 1}</small>
                  </button>
                ))}
              </div>
              <div className="touch-strip">{[1,2,3,4].map((n) => <div key={n}>Touch {n}</div>)}</div>
              <div className="dial-row">{[1,2,3,4].map((n) => <button data-testid="dial" key={n} className="dial"><span></span><small>Dial {n}</small></button>)}</div>
            </div>
            <div className="page-row"><button disabled>‹</button><span>Page 1 / 1</span><button disabled>›</button></div>
          </div>

          <section className="inspector">
            <div className="section-title"><h2>Property Inspector</h2><span>Key {selectedKey + 1}</span></div>
            {!selectedAction ? <p>Select an action from the Action List, then click this key to assign it.</p> : (
              <div className="inspector-fields">
                <label><span>Title</span><input value={selectedAction.label} onChange={(e) => updateSelected({ label: e.target.value })}/></label>
                {selectedAction.kind === 'obs.scene' && <label><span>Scene</span><select value={selectedAction.sceneName} onChange={(e) => updateSelected({ sceneName: e.target.value })}><option value="">Choose scene…</option>{scenes.map((scene) => <option key={scene}>{scene}</option>)}</select></label>}
                {selectedAction.kind === 'obs.toggleMute' && <label><span>OBS input name</span><input value={selectedAction.inputName} onChange={(e) => updateSelected({ inputName: e.target.value })}/></label>}
                <div className="inspector-actions"><button onClick={() => void execute(selectedAction)}>Test Action</button><button className="danger" onClick={() => setKeys((current) => current.map((value, i) => i === selectedKey ? null : value))}>Remove</button></div>
              </div>
            )}
          </section>
        </section>

        <aside className="action-list">
          <div className="section-title"><h2>Actions</h2><input aria-label="Search actions" placeholder="Search…" /></div>
          {Object.entries(actionGroups).map(([group, items]) => <section key={group} className="action-group"><h3>{group}</h3>{items.map((item) => <button key={item.id} onClick={() => { setPendingAction(item.action); setMessage(`Select a key for ${item.label}`); }}>{item.label}</button>)}</section>)}
        </aside>
      </main>

      <footer className="statusbar"><span>{message}</span><span>OpenDeck 2.0.0 Clean</span></footer>

      {panel !== 'none' && <div className="overlay" onMouseDown={() => setPanel('none')}><section className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <button className="close" aria-label="Close" onClick={() => setPanel('none')}>×</button>
        {panel === 'connections' ? <>
          <h2>Accounts & Connections</h2>
          <div className="connection-card"><h3>OBS Studio</h3><div className="row"><input value={obsHost} onChange={(e) => setObsHost(e.target.value)} aria-label="OBS host"/><input type="number" value={obsPort} onChange={(e) => setObsPort(Number(e.target.value))} aria-label="OBS port"/></div><input type="password" value={obsPassword} onChange={(e) => setObsPassword(e.target.value)} placeholder="WebSocket password" aria-label="OBS password"/><button onClick={() => void connectObs()}>Connect OBS</button><p>{obsStatus}</p></div>
          <div className="connection-card"><h3>Twitch</h3>{twitchIdentity ? <p>Signed in as <strong>{twitchIdentity.login}</strong></p> : <><button onClick={() => void beginTwitch()}>Sign in with Twitch</button><p>{twitchMessage}</p>{twitchCode && <code>{twitchCode.user_code}</code>}</>}</div>
        </> : <>
          <h2>Elgato Marketplace</h2><p>Open the official Marketplace in your browser. Compatible downloads are discovered locally; OpenDeck does not collect your Elgato password.</p><div className="row"><button onClick={() => void bridge.openExternal('https://marketplace.elgato.com')}>Open Marketplace</button><button onClick={() => void scanMarketplace()}>Scan Downloads</button></div><div className="market-list">{marketItems.length === 0 ? <p>No compatible downloaded items found yet.</p> : marketItems.map((item) => <article key={item.path}><strong>{item.name}</strong><span>{item.kind}</span><small>{item.path}</small></article>)}</div>
        </>}
      </section></div>}
    </div>
  );
}
