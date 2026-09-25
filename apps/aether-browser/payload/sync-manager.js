'use strict';
// AETHER_BROWSER_ENCRYPTED_SYNC_MANAGER_V3059
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { safeStorage } = require('electron');

const BUNDLE_NAME = 'AetherBrowser-Sync-v1.aes.json';
const META_SCHEMA = 1;
const BUNDLE_SCHEMA = 1;
const AUTO_INTERVAL_MS = 5 * 60 * 1000;

function clone(v) { return JSON.parse(JSON.stringify(v)); }
function sha256(v) { return crypto.createHash('sha256').update(typeof v === 'string' ? v : JSON.stringify(v)).digest('hex'); }
function atomicWrite(file, data, mode = 0o600) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const tmp = `${file}.tmp-${process.pid}-${Date.now()}`;
  fs.writeFileSync(tmp, data, { mode });
  fs.renameSync(tmp, file);
  try { fs.chmodSync(file, mode); } catch {}
}
function readJson(file, fallback = null) {
  try { return JSON.parse(fs.readFileSync(file, 'utf8')); } catch { return fallback; }
}
function safeMtime(file) {
  try { return Math.max(0, Number(fs.statSync(file).mtimeMs || 0)); } catch { return 0; }
}
function safeReadJson(file) {
  try { return JSON.parse(fs.readFileSync(file, 'utf8')); } catch { return null; }
}
function cleanSettings(settings) {
  const src = settings && typeof settings === 'object' ? settings : {};
  const out = {};
  for (const key of ['appearance', 'tabs', 'search']) {
    if (src[key] && typeof src[key] === 'object') out[key] = clone(src[key]);
  }
  return out;
}
function localStorageAllowlisted(obj) {
  const out = {};
  for (const [key, value] of Object.entries(obj || {})) {
    if (
      /^aether(?:\.|browser|Browser)/i.test(key) &&
      /(bookmark|speed.?dial|tab.?order|javascript|user.?script|site.?policy|start.?page)/i.test(key)
    ) out[key] = String(value);
  }
  return out;
}

class SyncManager {
  constructor({ profileRoot, sessionRoot, getSettings, applySettings, getWindows }) {
    this.profileRoot = profileRoot;
    this.sessionRoot = sessionRoot;
    this.getSettings = getSettings;
    this.applySettings = applySettings;
    this.getWindows = getWindows;
    this.syncStateRoot = path.join(profileRoot, 'sync-v1');
    this.metaFile = path.join(this.syncStateRoot, 'meta.json');
    this.secretFile = path.join(this.syncStateRoot, 'local-secret.bin');
    this.instanceFile = path.join(this.syncStateRoot, 'instance-id');
    this.passphrase = '';
    this.autoTimer = null;
    fs.mkdirSync(this.syncStateRoot, { recursive: true });
    this.instanceId = this.loadInstanceId();
    this.restoreRememberedPassphrase();
  }

  loadInstanceId() {
    try {
      const value = fs.readFileSync(this.instanceFile, 'utf8').trim();
      if (value) return value;
    } catch {}
    const value = crypto.randomUUID();
    atomicWrite(this.instanceFile, `${value}\n`, 0o600);
    return value;
  }

  restoreRememberedPassphrase() {
    try {
      if (!safeStorage.isEncryptionAvailable() || !fs.existsSync(this.secretFile)) return false;
      const buf = fs.readFileSync(this.secretFile);
      const plain = safeStorage.decryptString(buf);
      if (!plain) return false;
      this.passphrase = plain;
      return true;
    } catch {
      return false;
    }
  }

  unlock(passphrase, remember = true) {
    const value = String(passphrase || '');
    if (value.length < 8) return { ok: false, error: 'SYNC_PASSPHRASE_MINIMUM_8_CHARACTERS' };
    this.passphrase = value;
    let remembered = false;
    if (remember) {
      try {
        if (safeStorage.isEncryptionAvailable()) {
          atomicWrite(this.secretFile, safeStorage.encryptString(value), 0o600);
          remembered = true;
        }
      } catch {}
    }
    return { ok: true, unlocked: true, remembered };
  }

  lock() {
    this.passphrase = '';
    try { fs.rmSync(this.secretFile, { force: true }); } catch {}
    return { ok: true, unlocked: false };
  }

  settings() {
    return this.getSettings?.() || {};
  }

  syncDirectory() {
    const configured = String(this.settings()?.sync?.directory || '').trim();
    return configured;
  }

  bundlePath() {
    const dir = this.syncDirectory();
    return dir ? path.join(dir, BUNDLE_NAME) : '';
  }

  status() {
    const dir = this.syncDirectory();
    const bundle = this.bundlePath();
    const meta = readJson(this.metaFile, {}) || {};
    return {
      ok: true,
      enabled: Boolean(this.settings()?.sync?.enabled),
      directory: dir,
      unlocked: Boolean(this.passphrase),
      rememberedSecret: fs.existsSync(this.secretFile),
      bundlePath: bundle,
      bundleExists: Boolean(bundle && fs.existsSync(bundle)),
      lastSyncAt: Number(meta.lastSyncAt || 0),
      lastDirection: String(meta.lastDirection || ''),
      lastConflictAt: Number(meta.lastConflictAt || 0),
      instanceId: this.instanceId,
      encryption: 'AES-256-GCM+scrypt'
    };
  }

  async shellExport() {
    const states = this.getWindows?.() || [];
    const state = states.find((x) => x?.win && !x.win.isDestroyed?.());
    if (!state) return {};
    try {
      const raw = await state.win.webContents.executeJavaScript(`(() => {
        const out = {};
        for (let i = 0; i < localStorage.length; i += 1) {
          const key = localStorage.key(i);
          if (key != null) out[key] = localStorage.getItem(key);
        }
        return out;
      })()`, true);
      return localStorageAllowlisted(raw);
    } catch {
      return {};
    }
  }

  async shellImport(data) {
    const safe = localStorageAllowlisted(data);
    const states = this.getWindows?.() || [];
    const script = `(() => {
      const data = ${JSON.stringify(safe)};
      for (const [key, value] of Object.entries(data)) localStorage.setItem(key, String(value));
      return Object.keys(data).length;
    })()`;
    let applied = 0;
    for (const state of states) {
      if (!state?.win || state.win.isDestroyed?.()) continue;
      try { applied = Math.max(applied, Number(await state.win.webContents.executeJavaScript(script, true)) || 0); } catch {}
    }
    return applied;
  }

  async captureData() {
    const settings = this.settings();
    const categories = settings.sync || {};
    const data = {};
    const mtimes = {};

    if (categories.settings !== false) {
      data.settings = cleanSettings(settings);
      mtimes.settings = safeMtime(path.join(this.profileRoot, 'settings.json'));
    }
    if (categories.session !== false) {
      data.session = safeReadJson(path.join(this.sessionRoot, 'session-v1.json'));
      mtimes.session = safeMtime(path.join(this.sessionRoot, 'session-v1.json'));
    }
    if (categories.downloadHistory === true) {
      data.downloadHistory = safeReadJson(path.join(this.profileRoot, 'downloads-v1.json'));
      mtimes.downloadHistory = safeMtime(path.join(this.profileRoot, 'downloads-v1.json'));
    }
    if (categories.shellData !== false) {
      data.shellData = await this.shellExport();
      mtimes.shellData = Date.now();
    }

    return {
      schema: BUNDLE_SCHEMA,
      instanceId: this.instanceId,
      capturedAt: Date.now(),
      mtimes,
      data
    };
  }

  fingerprint(snapshot) {
    return sha256(snapshot?.data || {});
  }

  keyFrom(passphrase, salt) {
    return crypto.scryptSync(passphrase, salt, 32, { N: 16384, r: 8, p: 1 });
  }

  encrypt(snapshot) {
    if (!this.passphrase) throw new Error('SYNC_LOCKED');
    const salt = crypto.randomBytes(16);
    const iv = crypto.randomBytes(12);
    const key = this.keyFrom(this.passphrase, salt);
    const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);
    const plaintext = Buffer.from(JSON.stringify(snapshot), 'utf8');
    const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
    const tag = cipher.getAuthTag();
    return {
      format: 'AETHER_BROWSER_SYNC_AES256_GCM_V1',
      schema: BUNDLE_SCHEMA,
      kdf: 'scrypt-N16384-r8-p1',
      salt: salt.toString('base64'),
      iv: iv.toString('base64'),
      tag: tag.toString('base64'),
      ciphertext: ciphertext.toString('base64')
    };
  }

  decrypt(wrapper) {
    if (!this.passphrase) throw new Error('SYNC_LOCKED');
    if (!wrapper || wrapper.format !== 'AETHER_BROWSER_SYNC_AES256_GCM_V1') throw new Error('SYNC_BUNDLE_FORMAT_UNSUPPORTED');
    const salt = Buffer.from(String(wrapper.salt || ''), 'base64');
    const iv = Buffer.from(String(wrapper.iv || ''), 'base64');
    const tag = Buffer.from(String(wrapper.tag || ''), 'base64');
    const ciphertext = Buffer.from(String(wrapper.ciphertext || ''), 'base64');
    const key = this.keyFrom(this.passphrase, salt);
    const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv);
    decipher.setAuthTag(tag);
    const plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]);
    const parsed = JSON.parse(plaintext.toString('utf8'));
    if (!parsed || parsed.schema !== BUNDLE_SCHEMA || !parsed.data) throw new Error('SYNC_PAYLOAD_INVALID');
    return parsed;
  }

  readRemote() {
    const bundle = this.bundlePath();
    if (!bundle || !fs.existsSync(bundle)) return null;
    return this.decrypt(readJson(bundle, null));
  }

  writeRemote(snapshot) {
    const bundle = this.bundlePath();
    if (!bundle) throw new Error('SYNC_DIRECTORY_REQUIRED');
    fs.mkdirSync(path.dirname(bundle), { recursive: true });
    atomicWrite(bundle, `${JSON.stringify(this.encrypt(snapshot), null, 2)}\n`, 0o600);
    return bundle;
  }

  meta() {
    return readJson(this.metaFile, { schema: META_SCHEMA }) || { schema: META_SCHEMA };
  }

  writeMeta(patch) {
    const current = this.meta();
    const next = { ...current, ...patch, schema: META_SCHEMA };
    atomicWrite(this.metaFile, `${JSON.stringify(next, null, 2)}\n`, 0o600);
    return next;
  }

  async exportLocal() {
    if (!this.passphrase) return { ok: false, error: 'SYNC_LOCKED' };
    if (!this.syncDirectory()) return { ok: false, error: 'SYNC_DIRECTORY_REQUIRED' };
    const local = await this.captureData();
    const fp = this.fingerprint(local);
    const bundle = this.writeRemote(local);
    this.writeMeta({
      lastLocalFingerprint: fp,
      lastRemoteFingerprint: fp,
      lastSyncAt: Date.now(),
      lastDirection: 'export'
    });
    return { ok: true, direction: 'export', bundlePath: bundle, fingerprint: fp };
  }

  async applyRemoteSnapshot(remote) {
    const data = remote?.data || {};
    let restartRequired = false;

    if (data.settings && typeof data.settings === 'object') {
      this.applySettings?.(data.settings);
    }
    if (data.session && typeof data.session === 'object') {
      atomicWrite(path.join(this.sessionRoot, 'session-v1.json'), `${JSON.stringify(data.session, null, 2)}\n`, 0o600);
      restartRequired = true;
    }
    if (data.downloadHistory && typeof data.downloadHistory === 'object') {
      atomicWrite(path.join(this.profileRoot, 'downloads-v1.json'), `${JSON.stringify(data.downloadHistory, null, 2)}\n`, 0o600);
    }
    if (data.shellData && typeof data.shellData === 'object') {
      await this.shellImport(data.shellData);
    }
    return { restartRequired };
  }

  async importRemote() {
    if (!this.passphrase) return { ok: false, error: 'SYNC_LOCKED' };
    if (!this.syncDirectory()) return { ok: false, error: 'SYNC_DIRECTORY_REQUIRED' };
    let remote;
    try { remote = this.readRemote(); }
    catch (error) { return { ok: false, error: `SYNC_DECRYPT_FAILED:${String(error?.message || error)}` }; }
    if (!remote) return { ok: false, error: 'SYNC_REMOTE_BUNDLE_NOT_FOUND' };
    const fp = this.fingerprint(remote);
    const applied = await this.applyRemoteSnapshot(remote);
    this.writeMeta({
      lastLocalFingerprint: fp,
      lastRemoteFingerprint: fp,
      lastSyncAt: Date.now(),
      lastDirection: 'import'
    });
    return { ok: true, direction: 'import', fingerprint: fp, ...applied };
  }

  async syncNow() {
    const settings = this.settings();
    if (!settings?.sync?.enabled) return { ok: false, error: 'SYNC_DISABLED' };
    if (!this.passphrase) return { ok: false, error: 'SYNC_LOCKED' };
    if (!this.syncDirectory()) return { ok: false, error: 'SYNC_DIRECTORY_REQUIRED' };

    const local = await this.captureData();
    const localFp = this.fingerprint(local);
    let remote = null;
    try { remote = this.readRemote(); }
    catch (error) { return { ok: false, error: `SYNC_DECRYPT_FAILED:${String(error?.message || error)}` }; }

    if (!remote) return await this.exportLocal();

    const remoteFp = this.fingerprint(remote);
    const meta = this.meta();
    const priorLocal = String(meta.lastLocalFingerprint || '');
    const priorRemote = String(meta.lastRemoteFingerprint || '');

    if (!priorLocal && !priorRemote) {
      if (localFp === remoteFp) {
        this.writeMeta({
          lastLocalFingerprint: localFp,
          lastRemoteFingerprint: remoteFp,
          lastSyncAt: Date.now(),
          lastDirection: 'matched'
        });
        return { ok: true, direction: 'matched' };
      }
      this.writeMeta({ lastConflictAt: Date.now(), lastDirection: 'first-sync-conflict' });
      return { ok: false, conflict: true, error: 'SYNC_FIRST_RUN_CONFLICT', localFingerprint: localFp, remoteFingerprint: remoteFp };
    }

    const localChanged = localFp !== priorLocal;
    const remoteChanged = remoteFp !== priorRemote;

    if (localChanged && remoteChanged && localFp !== remoteFp) {
      const conflictDir = path.join(this.syncDirectory(), 'AetherBrowser-Sync-Conflicts');
      fs.mkdirSync(conflictDir, { recursive: true });
      const stamp = new Date().toISOString().replace(/[:.]/g, '-');
      atomicWrite(path.join(conflictDir, `${stamp}-${this.instanceId}-LOCAL.json`), `${JSON.stringify(this.encrypt(local), null, 2)}\n`, 0o600);
      atomicWrite(path.join(conflictDir, `${stamp}-${remote.instanceId || 'REMOTE'}-REMOTE.json`), `${JSON.stringify(this.encrypt(remote), null, 2)}\n`, 0o600);
      this.writeMeta({ lastConflictAt: Date.now(), lastDirection: 'conflict' });
      return { ok: false, conflict: true, error: 'SYNC_CONFLICT_REQUIRES_CHOICE', localFingerprint: localFp, remoteFingerprint: remoteFp };
    }

    if (remoteChanged && !localChanged) return await this.importRemote();
    if (localChanged && !remoteChanged) return await this.exportLocal();

    this.writeMeta({
      lastLocalFingerprint: localFp,
      lastRemoteFingerprint: remoteFp,
      lastSyncAt: Date.now(),
      lastDirection: localFp === remoteFp ? 'matched' : 'noop'
    });
    return { ok: true, direction: localFp === remoteFp ? 'matched' : 'noop' };
  }

  startAuto() {
    if (this.autoTimer) return;
    this.autoTimer = setInterval(() => {
      if (!this.settings()?.sync?.enabled || !this.passphrase || !this.syncDirectory()) return;
      this.syncNow().catch(() => {});
    }, AUTO_INTERVAL_MS);
    this.autoTimer.unref?.();
  }

  stopAuto() {
    if (this.autoTimer) clearInterval(this.autoTimer);
    this.autoTimer = null;
  }

  async command(name, payload = {}) {
    if (name === 'status') return this.status();
    if (name === 'unlock') return this.unlock(payload.passphrase, payload.remember !== false);
    if (name === 'lock') return this.lock();
    if (name === 'sync-now') return await this.syncNow();
    if (name === 'export-local') return await this.exportLocal();
    if (name === 'import-remote') return await this.importRemote();
    return { ok: false, error: 'SYNC_COMMAND_UNKNOWN' };
  }
}

module.exports = { SyncManager };
