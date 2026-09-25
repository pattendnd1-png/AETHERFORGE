'use strict';
// AETHER_BROWSER_PATCH_V3059
const fs = require('node:fs');
const path = require('node:path');

const root = process.argv[2];
if (!root) throw new Error('TARGET_APP_REQUIRED');

const VERSION = '3.0.59';
const BASE = '3.0.58';
const src = path.join(root, 'src');
const ui = path.join(root, 'ui');

function must(rel) {
  const p = path.join(root, rel);
  if (!fs.existsSync(p)) throw new Error('v3059-required-path-missing:' + rel);
  return p;
}

[
  'package.json','VERSION',
  'src/runtime-v3.0.58.js','src/bootstrap.js','src/preload.js',
  'src/settings-store.js','src/settings-preload.js',
  'src/context-menu.js','src/torrent-manager.js','src/streaming-integrations.js',
  'src/download-manager.js','src/protected-media.js','src/javascript-suite.js',
  'ui/index.html','ui/aether-settings.html','ui/aether-v3.0.32-settings-tab.js',
  'ui/aether-v3.0.58-tab-detach.js','ui/aether-v3.0.58-tab-detach.css',
  'ui/aether-v3.0.56-media-download.css','ui/aether-v3.0.53-solid-shell.css'
].forEach(must);

const pkgPath = path.join(root, 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
if (pkg.version !== BASE || pkg.main !== 'src/bootstrap.js') {
  throw new Error(`v3059-unexpected-base-package:${pkg.version}:${pkg.main}`);
}
pkg.version = VERSION;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
fs.writeFileSync(path.join(root, 'VERSION'), VERSION + '\n');

// Preserve v3.0.58 feature ownership before modifying anything.
const baseRt = fs.readFileSync(path.join(src, 'runtime-v3.0.58.js'), 'utf8');
for (const marker of [
  'AETHER_BROWSER_TRUE_HTML_FULLSCREEN_V3053',
  'AETHER_BROWSER_TUBI_HIGHEST_QUALITY_V3053',
  'AETHER_BROWSER_DRM_RUNTIME_PROBE_V3053',
  'AETHER_BROWSER_PROTECTED_CONTENT_PERMISSION_V3053',
  'AETHER_BROWSER_UNIFIED_DOWNLOAD_CENTER_V3056',
  'AETHER_BROWSER_TAB_DETACH_RUNTIME_V3058',
  'AETHER_BROWSER_DETACH_CLOSE_HISTORY_GUARD_V3058',
  'AETHER_BROWSER_LOGIN_POPUP_NAVIGATION_V3058',
  "case 'tab-detach':",
  'setWindowOpenHandler'
]) {
  if (!baseRt.includes(marker)) throw new Error('v3059-base-runtime-feature-missing:' + marker);
}
if (!fs.readFileSync(path.join(src, 'context-menu.js'), 'utf8').includes('AETHER_BROWSER_NATIVE_CONTEXT_MENUS_V3058')) throw new Error('v3059-base-context-menu-missing');
if (!fs.readFileSync(path.join(src, 'torrent-manager.js'), 'utf8').includes('AETHER_BROWSER_TORRENT_MANAGER_V3056')) throw new Error('v3059-base-torrent-manager-missing');
if (!fs.readFileSync(path.join(src, 'streaming-integrations.js'), 'utf8').includes('AETHER_BROWSER_STREAMING_INTEGRATIONS_V3056')) throw new Error('v3059-base-streaming-integrations-missing');

// Settings schema: search engine + sync configuration.
const settingsPath = path.join(src, 'settings-store.js');
let settings = fs.readFileSync(settingsPath, 'utf8');
if (!settings.includes('AETHER_BROWSER_SEARCH_SYNC_SETTINGS_V3059')) {
  const defaultsAnchor = "  downloads: Object.freeze({ directory: '' })";
  if (!settings.includes(defaultsAnchor)) throw new Error('v3059-settings-defaults-anchor-missing');
  settings = settings.replace(defaultsAnchor, `${defaultsAnchor},
  // AETHER_BROWSER_SEARCH_SYNC_SETTINGS_V3059
  search: Object.freeze({ defaultEngine: 'google' }),
  sync: Object.freeze({
    enabled: false,
    directory: '',
    settings: true,
    session: true,
    shellData: true,
    downloadHistory: false
  })`);

  const ruleAnchor = /('downloads\.directory'\s*:\s*\(v\)\s*=>[^\n]+)(\n\}\);)/;
  if (!ruleAnchor.test(settings)) throw new Error('v3059-settings-rules-anchor-missing');
  settings = settings.replace(ruleAnchor, `$1,
  'search.defaultEngine': (v) => ['google','duckduckgo','yahoo','bing','brave','perplexity','you','phind'].includes(v),
  'sync.enabled': (v) => typeof v === 'boolean',
  'sync.directory': (v) => typeof v === 'string' && v.length <= 4096 && !/\\0/.test(v),
  'sync.settings': (v) => typeof v === 'boolean',
  'sync.session': (v) => typeof v === 'boolean',
  'sync.shellData': (v) => typeof v === 'boolean',
  'sync.downloadHistory': (v) => typeof v === 'boolean'$2`);
}
fs.writeFileSync(settingsPath, settings);

// Runtime: search engine routing + encrypted sync manager.
const oldRuntime = path.join(src, 'runtime-v3.0.58.js');
const newRuntime = path.join(src, 'runtime-v3.0.59.js');
fs.copyFileSync(oldRuntime, newRuntime);
let runtime = fs.readFileSync(newRuntime, 'utf8');

if (!runtime.includes("require('./sync-manager')")) {
  const anchor = "const { installAetherContextMenus } = require('./context-menu');";
  if (!runtime.includes(anchor)) throw new Error('v3059-sync-import-anchor-missing');
  runtime = runtime.replace(anchor, `${anchor}\nconst { SyncManager } = require('./sync-manager');`);
}

// Search provider wrapper. Keep old URL parser as the source of truth for URL-like input.
if (!runtime.includes('AETHER_BROWSER_SEARCH_ENGINES_V3059')) {
  const original = 'function normalizeInput(';
  const count = runtime.split(original).length - 1;
  if (count !== 1) throw new Error('v3059-normalize-input-anchor-count:' + count);
  runtime = runtime.replace(original, 'function aetherBaseNormalizeInputV3059(');
  const renamed = 'function aetherBaseNormalizeInputV3059(';
  const helper = `// AETHER_BROWSER_SEARCH_ENGINES_V3059
const AETHER_SEARCH_ENGINES_V3059 = Object.freeze({
  google: (q) => 'https://www.google.com/search?q=' + encodeURIComponent(q),
  duckduckgo: (q) => 'https://duckduckgo.com/?q=' + encodeURIComponent(q),
  yahoo: (q) => 'https://search.yahoo.com/search?p=' + encodeURIComponent(q),
  bing: (q) => 'https://www.bing.com/search?q=' + encodeURIComponent(q),
  brave: (q) => 'https://search.brave.com/search?q=' + encodeURIComponent(q),
  perplexity: (q) => 'https://www.perplexity.ai/search?q=' + encodeURIComponent(q),
  you: (q) => 'https://you.com/?q=' + encodeURIComponent(q),
  phind: (q) => 'https://www.phind.com/search?q=' + encodeURIComponent(q)
});
function aetherShouldSearchV3059(raw) {
  const value = String(raw || '').trim();
  if (!value || value === START_PAGE_TOKEN || value === 'about:blank') return false;
  if (/^[a-zA-Z][a-zA-Z0-9+.-]*:\\/\\//.test(value)) return false;
  if (/^(?:file|about|mailto|magnet):/i.test(value)) return false;
  if (/^localhost(?::\\d+)?(?:[/?#]|$)/i.test(value)) return false;
  if (/^\\d{1,3}(?:\\.\\d{1,3}){3}(?::\\d+)?(?:[/?#]|$)/.test(value)) return false;
  if (/^\\[[0-9a-f:]+\\](?::\\d+)?(?:[/?#]|$)/i.test(value)) return false;
  if (!/\\s/.test(value) && /^[^/?#\\s]+\\.[^\\s]+/.test(value)) return false;
  return true;
}
function aetherSearchUrlV3059(query) {
  const selected = String(ACTIVE_SETTINGS?.search?.defaultEngine || 'google');
  const build = AETHER_SEARCH_ENGINES_V3059[selected] || AETHER_SEARCH_ENGINES_V3059.google;
  return build(String(query || '').trim());
}
function normalizeInput(raw) {
  if (aetherShouldSearchV3059(raw)) return aetherSearchUrlV3059(raw);
  return aetherBaseNormalizeInputV3059(raw);
}

`;
  runtime = runtime.replace(renamed, helper + renamed);
}

if (!runtime.includes('AETHER_BROWSER_SYNC_RUNTIME_V3059')) {
  const windowsAnchor = 'const windows = new Set();';
  if (!runtime.includes(windowsAnchor)) throw new Error('v3059-windows-anchor-missing');
  const block = `${windowsAnchor}
// AETHER_BROWSER_SYNC_RUNTIME_V3059
const SYNC_MANAGER = new SyncManager({
  profileRoot: PROFILE_ROOT,
  sessionRoot: SESSION_ROOT,
  getSettings: () => ACTIVE_SETTINGS,
  applySettings: (incoming) => {
    const current = JSON.parse(JSON.stringify(ACTIVE_SETTINGS || {}));
    for (const key of ['appearance','tabs','search']) {
      if (incoming && incoming[key] && typeof incoming[key] === 'object') current[key] = incoming[key];
    }
    ACTIVE_SETTINGS = SETTINGS_STORE.write(current);
    try { broadcastSettingsChanged(ACTIVE_SETTINGS); } catch {}
    return ACTIVE_SETTINGS;
  },
  getWindows: () => [...windows]
});
`;
  runtime = runtime.replace(windowsAnchor, block);
}

if (!runtime.includes("ipcMain.handle('aether:sync-command'")) {
  const ipcAnchor = "ipcMain.handle('aether:settings:diagnostics', () => settingsDiagnostics());";
  if (!runtime.includes(ipcAnchor)) throw new Error('v3059-sync-ipc-anchor-missing');
  runtime = runtime.replace(ipcAnchor, `${ipcAnchor}
ipcMain.handle('aether:sync-command', async (_event, name, payload) => {
  try { return await SYNC_MANAGER.command(String(name || ''), payload || {}); }
  catch (error) { return { ok: false, error: String(error && error.message || error) }; }
});`);
}

// Make live-reload cleanup aware of the new IPC channel.
const channelsRe = /const RUNTIME_IPC_CHANNELS=\[([^\]]*)\];/;
const m = runtime.match(channelsRe);
if (!m) throw new Error('v3059-runtime-ipc-channels-anchor-missing');
if (!m[1].includes("'aether:sync-command'")) {
  const items = m[1].trim();
  runtime = runtime.replace(channelsRe, `const RUNTIME_IPC_CHANNELS=[${items}${items ? ',' : ''}'aether:sync-command'];`);
}

// Start/stop auto sync with the runtime lifecycle.
if (!runtime.includes('AETHER_BROWSER_SYNC_AUTO_START_V3059')) {
  const startNeedle = "async function startRuntime(options={}){";
  if (!runtime.includes(startNeedle)) throw new Error('v3059-start-runtime-anchor-missing');
  runtime = runtime.replace(startNeedle, `async function startRuntime(options={}){// AETHER_BROWSER_SYNC_AUTO_START_V3059\nSYNC_MANAGER.startAuto();`);
}
if (!runtime.includes('AETHER_BROWSER_SYNC_AUTO_STOP_V3059')) {
  const stopNeedle = "async function stopRuntime({reason='hot-reload'}={}){";
  if (!runtime.includes(stopNeedle)) throw new Error('v3059-stop-runtime-anchor-missing');
  runtime = runtime.replace(stopNeedle, `async function stopRuntime({reason='hot-reload'}={}){// AETHER_BROWSER_SYNC_AUTO_STOP_V3059\nSYNC_MANAGER.stopAuto();`);
}

fs.writeFileSync(newRuntime, runtime);

// Settings preload gets narrow sync commands.
const spreloadPath = path.join(src, 'settings-preload.js');
let spreload = fs.readFileSync(spreloadPath, 'utf8');
if (!spreload.includes('syncStatus:')) {
  const diag = "  diagnostics: () => ipcRenderer.invoke('aether:settings:diagnostics')";
  if (!spreload.includes(diag)) throw new Error('v3059-settings-preload-anchor-missing');
  spreload = spreload.replace(diag, `${diag},
  syncStatus: () => ipcRenderer.invoke('aether:sync-command', 'status'),
  syncUnlock: (passphrase, remember) => ipcRenderer.invoke('aether:sync-command', 'unlock', { passphrase, remember }),
  syncLock: () => ipcRenderer.invoke('aether:sync-command', 'lock'),
  syncNow: () => ipcRenderer.invoke('aether:sync-command', 'sync-now'),
  syncExportLocal: () => ipcRenderer.invoke('aether:sync-command', 'export-local'),
  syncImportRemote: () => ipcRenderer.invoke('aether:sync-command', 'import-remote')`);
}
fs.writeFileSync(spreloadPath, spreload);

// Settings page: Search + Sync.
const settingsHtmlPath = path.join(ui, 'aether-settings.html');
let html = fs.readFileSync(settingsHtmlPath, 'utf8');

if (!html.includes('data-settings-section="search"')) {
  const anchor = '<button data-settings-section="privacy">Privacy & Security</button>';
  if (!html.includes(anchor)) throw new Error('v3059-settings-nav-anchor-missing');
  html = html.replace(anchor, `<button data-settings-section="search">Search</button><button data-settings-section="sync">Sync</button>${anchor}`);
}

if (!html.includes('data-settings-page="search"')) {
  const anchor = '<section class="settings-section" data-settings-page="privacy">';
  if (!html.includes(anchor)) throw new Error('v3059-settings-page-anchor-missing');
  const sections = `<section class="settings-section" data-settings-page="search"><h2>Search</h2>
    <p class="settings-lede">Choose which search engine handles non-URL text entered in the omnibar.</p>
    <label>Default search engine
      <select data-setting="search.defaultEngine">
        <option value="google">Google</option>
        <option value="duckduckgo">DuckDuckGo</option>
        <option value="yahoo">Yahoo</option>
        <option value="bing">Bing</option>
        <option value="brave">Brave Search</option>
        <option value="perplexity">Perplexity — AI</option>
        <option value="you">You.com — AI</option>
        <option value="phind">Phind — AI</option>
      </select>
    </label>
    <div class="search-engine-grid">
      <div class="search-engine-chip"><b>Traditional</b><span>Google · DuckDuckGo · Yahoo · Bing · Brave</span></div>
      <div class="search-engine-chip"><b>AI powered</b><span>Perplexity · You.com · Phind</span></div>
    </div>
  </section>
  <section class="settings-section" data-settings-page="sync"><h2>Browser Sync</h2>
    <p class="settings-lede">Encrypted, provider-neutral sync for browser data. Cookies, passwords, OAuth tokens and Widevine state are never included.</p>
    <label class="settings-check"><input data-setting="sync.enabled" type="checkbox"> Enable automatic browser sync</label>
    <label>Sync folder<input data-setting="sync.directory" type="text" placeholder="Example: /home/you/Sync/AetherBrowser"></label>
    <label class="settings-check"><input data-setting="sync.settings" type="checkbox"> Sync appearance, tab preferences and search engine</label>
    <label class="settings-check"><input data-setting="sync.session" type="checkbox"> Sync open tabs / window session</label>
    <label class="settings-check"><input data-setting="sync.shellData" type="checkbox"> Sync bookmarks, speed dial and browser UI data</label>
    <label class="settings-check"><input data-setting="sync.downloadHistory" type="checkbox"> Sync download-history metadata</label>
    <label>Sync passphrase<input id="syncPassphrase" type="password" autocomplete="new-password" placeholder="At least 8 characters — never written to the sync bundle"></label>
    <label class="settings-check"><input id="syncRememberPassphrase" type="checkbox" checked> Remember passphrase on this machine using the OS key store</label>
    <div class="sync-actions">
      <button id="syncUnlockButton" type="button">Unlock</button>
      <button id="syncLockButton" type="button">Lock</button>
      <button id="syncNowButton" type="button">Sync Now</button>
      <button id="syncExportButton" type="button">Export Local</button>
      <button id="syncImportButton" type="button">Import Remote</button>
    </div>
    <div class="sync-status-grid">
      <div><span>Encryption</span><strong>AES-256-GCM + scrypt</strong></div>
      <div><span>State</span><strong id="syncStatusUnlocked">Checking…</strong></div>
      <div><span>Remote bundle</span><strong id="syncStatusBundle">Checking…</strong></div>
      <div><span>Last sync</span><strong id="syncStatusLast">Checking…</strong></div>
      <div><span>Last action</span><strong id="syncStatusDirection">Checking…</strong></div>
    </div>
    <p id="syncActionStatus" class="note" aria-live="polite"></p>
    <p class="note">If both devices change since the previous sync, AetherBrowser stops and asks you to choose Import Remote or Export Local rather than overwriting data silently.</p>
  </section>
  ${anchor}`;
  html = html.replace(anchor, sections);
}

if (!html.includes('aether-v3.0.59-search-sync.css')) {
  html = html.replace('</head>', '<link rel="stylesheet" href="aether-v3.0.59-search-sync.css"></head>');
}
if (!html.includes('aether-v3.0.59-search-sync.js')) {
  html = html.replace('</body>', '<script src="./aether-v3.0.59-search-sync.js"></script></body>');
}
html = html.replace(/(<strong\s+id=["']diagVersion["'][^>]*>)[^<]*(<\/strong>)/i, (_m,a,b)=>`${a}${VERSION}${b}`);
fs.writeFileSync(settingsHtmlPath, html);

for (const name of fs.readdirSync(ui).filter((x) => /settings.*\.js$|\.settings\.js$/i.test(x))) {
  const p = path.join(ui, name);
  if (name === 'aether-v3.0.59-search-sync.js') continue;
  let t = fs.readFileSync(p, 'utf8');
  t = t.replace(/(diagVersion[^\n]*?d\.version\s*\|\|\s*['"])(3\.0\.\d+)(['"])/g, `$1${VERSION}$3`);
  fs.writeFileSync(p, t);
}

console.log('AETHER_BROWSER_V3059_PATCH=PASS');
