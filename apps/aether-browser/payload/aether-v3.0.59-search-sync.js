'use strict';
// AETHER_BROWSER_SEARCH_SYNC_SETTINGS_UI_V3059
(() => {
  const $ = (id) => document.getElementById(id);
  const api = window.aetherSettings;

  function status(message, bad = false) {
    const el = $('syncActionStatus');
    if (!el) return;
    el.textContent = String(message || '');
    el.dataset.error = bad ? '1' : '0';
  }

  async function refresh() {
    const s = await api?.syncStatus?.();
    if (!s?.ok) return;
    if ($('syncStatusUnlocked')) $('syncStatusUnlocked').textContent = s.unlocked ? 'Unlocked' : 'Locked';
    if ($('syncStatusBundle')) $('syncStatusBundle').textContent = s.bundleExists ? 'Bundle ready' : 'No bundle yet';
    if ($('syncStatusLast')) $('syncStatusLast').textContent = s.lastSyncAt ? new Date(s.lastSyncAt).toLocaleString() : 'Never';
    if ($('syncStatusDirection')) $('syncStatusDirection').textContent = s.lastDirection || '—';
  }

  $('syncUnlockButton')?.addEventListener('click', async () => {
    const input = $('syncPassphrase');
    const passphrase = String(input?.value || '');
    const result = await api?.syncUnlock?.(passphrase, Boolean($('syncRememberPassphrase')?.checked));
    if (result?.ok) {
      if (input) input.value = '';
      status(result.remembered ? 'Sync unlocked. Passphrase protected by the OS key store.' : 'Sync unlocked for this session.');
      await refresh();
    } else status(result?.error || 'Could not unlock sync.', true);
  });

  $('syncLockButton')?.addEventListener('click', async () => {
    const result = await api?.syncLock?.();
    status(result?.ok ? 'Sync locked and remembered secret removed.' : (result?.error || 'Could not lock sync.'), !result?.ok);
    await refresh();
  });

  async function run(kind) {
    status('Working…');
    const fn = kind === 'sync' ? api?.syncNow : kind === 'export' ? api?.syncExportLocal : api?.syncImportRemote;
    const result = await fn?.();
    if (result?.ok) {
      const restart = result.restartRequired ? ' Restart AetherBrowser to apply synced tab/session state.' : '';
      status(`Sync ${result.direction || kind} completed.${restart}`);
    } else if (result?.conflict) {
      status('Both this browser and the remote bundle changed. Choose Export Local or Import Remote.', true);
    } else {
      status(result?.error || 'Sync failed.', true);
    }
    await refresh();
  }

  $('syncNowButton')?.addEventListener('click', () => void run('sync'));
  $('syncExportButton')?.addEventListener('click', () => void run('export'));
  $('syncImportButton')?.addEventListener('click', () => void run('import'));

  window.addEventListener('DOMContentLoaded', () => void refresh(), { once: true });
  window.__AETHER_BROWSER_SEARCH_SYNC_SETTINGS_UI_V3059__ = true;
})();
