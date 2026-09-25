# AetherBrowser v3.0.59 — Search Engine Chooser + Encrypted Browser Sync

Base: the working v3.0.58 release.

## Search

AetherBrowser Settings now has a Search section. Non-URL text typed in the
omnibar uses the selected default engine.

Built-in engines:
- Google
- DuckDuckGo
- Yahoo
- Bing
- Brave Search
- Perplexity
- You.com
- Phind

URL-like input remains direct navigation. The search setting is persisted in
the normal AetherBrowser settings store and applies browser-wide.

## Browser Sync

The Sync section provides an encrypted, provider-neutral sync bundle in a
folder chosen by the user.

The portable sync bundle uses:
- AES-256-GCM authenticated encryption
- scrypt passphrase derivation
- a random salt and IV per write
- atomic file replacement
- a per-browser instance identifier
- fingerprint-based conflict detection

The passphrase is never written into the portable sync bundle. When the user
chooses "remember", AetherBrowser stores the passphrase only on the local
machine using Electron safeStorage / the operating-system credential
protection when available.

Syncable categories:
- appearance/tab/search settings
- open tabs / saved window session
- allowlisted browser-shell data such as bookmarks/speed dial/tab order
- optional download-history metadata

Explicitly excluded:
- cookies
- passwords
- OAuth/login tokens
- Widevine/CDM state
- web cache
- downloaded files
- torrent payload data

Conflict policy:
- local-only changes: export
- remote-only changes: import
- no changes: no-op
- both changed: STOP and require explicit "Export Local" or "Import Remote"

The first sync against a different existing bundle also requires an explicit
choice instead of overwriting either side.

Automatic sync runs only while AetherBrowser is running, Sync is enabled, a
folder is configured, and the sync passphrase is unlocked.
