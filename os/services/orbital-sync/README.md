# AETHERFORGE Orbital Sync

Single-authority AutoScan + AutoUpload for the post-reset Garuda-backed AETHERFORGE Rust reforge.

- Fast orbit: every 30 seconds for canonical repo changes and queued pushes.
- Full orbit: every 5 minutes for source rediscovery, OS/app import, artifact indexing/upload, and README/profile progress.
- One shared lock; no overlapping syncs.
- No force pushes.
- Paste-friendly status: `~/Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt`.

The legacy timers are retired as schedulers; their worker binaries are invoked only by the orbital orchestrator.
