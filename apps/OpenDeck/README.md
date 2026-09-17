# OpenDeck+ 2.0.38 — System-Wide Full Install

OpenDeck+ 2.0.38 keeps the qualified 2.0.37 runtime intact and changes the installation/cutover model from a per-user replacement install to a true system-wide installation.

The qualified application payload is staged under `/opt/opendeck-plus/2.0.38`. The complete versioned tree contains the executable, desktop template, icon, Stream Deck+ udev rule, rollback/uninstall helpers, install metadata, and a SHA256 payload manifest. `/opt/opendeck-plus/current` is the stable active target; `/usr/local/bin/opendeck-studio` and `/usr/local/bin/opendeck` resolve through that link. The canonical application-menu entry is `/usr/share/applications/opendeck-studio.desktop`, the application icon is installed under `/usr/share/icons/hicolor/64x64/apps`, and device permissions are registered through `/etc/udev/rules.d/70-opendeck-streamdeck.rules`.

The host qualifier does not activate the new system integration immediately. It first runs the inherited frontend, strict Rust/Clippy, Tauri, Stream Deck+, and visual gates, then uses sudo to stage the versioned `/opt` tree. A temporary system desktop entry launches that installed `/opt` copy and must produce the normal Tauri native/frontend visibility acknowledgement. Only then does activation switch the stable system paths, back up every affected system/user launcher path, remove the old per-user launcher shadows, refresh desktop/KDE/icon/udev state, and test the canonical system application-menu entry again.

If a post-activation gate fails, the qualifier automatically invokes the recorded rollback state. The previous 2.0.37 user install tree is not deleted. A successful install provides `sudo /usr/local/sbin/opendeck-rollback /var/lib/opendeck-plus/rollback-v2.0.38` for rollback and `sudo /usr/local/sbin/opendeck-uninstall` to remove 2.0.38 and restore the previous integration. User profiles/settings remain in their existing per-user XDG locations and are not removed by the system installer.

OpenDeck+ 2.0.38 installs no background service and does not enable autostart.
