mod app;
mod health;
#[cfg(target_os = "linux")]
mod native_ui;
#[cfg(target_os = "windows")]
mod platform;
mod terminal;
mod update;
use aether_lifecycle::{FsLifecycleManager, StartupRecovery};
use app::{AetherApp, import_model_record};
use health::create_startup_checkpoint;
use std::env;
use std::path::{Path, PathBuf};
fn pending_managed_startup(
    manager: &FsLifecycleManager,
    diagnostic: bool,
    import: &Option<PathBuf>,
    import_chatgpt: &Option<PathBuf>,
) -> bool {
    !diagnostic
        && import.is_none()
        && import_chatgpt.is_none()
        && manager.current_executable_is_managed().unwrap_or(false)
        && manager.pending_startup_version().ok().flatten().is_some()
}

fn recover_pending_startup(
    manager: &FsLifecycleManager,
    reason: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(recovery) = manager.recover_pending_startup_failure(reason)? else {
        return Ok(false);
    };

    match recovery {
        StartupRecovery::RolledBack {
            failed_version,
            restored_version,
            binary,
            report,
        } => {
            println!("AETHERAI_STARTUP_RECOVERY=ROLLED_BACK");
            println!("AETHERAI_FAILED_VERSION={failed_version}");
            println!("AETHERAI_RESTORED_VERSION={restored_version}");
            println!("AETHERAI_RECOVERY_REPORT={}", report.display());
            std::process::Command::new(&binary).spawn()?;
            Ok(true)
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut diagnostic = false;
    let mut data_dir: Option<PathBuf> = None;
    let mut import: Option<PathBuf> = None;
    let mut import_chatgpt: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--diagnostic" => diagnostic = true,
            "--data-dir" => data_dir = args.next().map(PathBuf::from),
            "--import-model" => import = args.next().map(PathBuf::from),
            "--import-chatgpt" => import_chatgpt = args.next().map(PathBuf::from),
            _ => {}
        }
    }
    let dir = data_dir.unwrap_or_else(default_data_dir);
    let lifecycle = FsLifecycleManager::for_user().ok();
    let startup_pending = lifecycle.as_ref().is_some_and(|manager| {
        pending_managed_startup(manager, diagnostic, &import, &import_chatgpt)
    });

    if startup_pending {
        if let Err(error) = create_startup_checkpoint(&dir) {
            if let Some(manager) = lifecycle.as_ref() {
                if recover_pending_startup(
                    manager,
                    &format!("pre-startup database checkpoint failed: {error}"),
                )? {
                    return Ok(());
                }
            }
            return Err(error.into());
        }
    }

    let mut app = match AetherApp::new(&dir) {
        Ok(app) => app,
        Err(error) => {
            if startup_pending {
                if let Some(manager) = lifecycle.as_ref() {
                    if recover_pending_startup(
                        manager,
                        &format!("application initialization failed: {error}"),
                    )? {
                        return Ok(());
                    }
                }
            }
            return Err(error);
        }
    };

    if startup_pending && !app.health.startup_minimum_healthy() {
        let reason = format!(
            "minimum startup health failed before renderer: {}",
            app.health.summary()
        );
        if let Some(manager) = lifecycle.as_ref() {
            if recover_pending_startup(manager, &reason)? {
                return Ok(());
            }
        }
        return Err(reason.into());
    }

    app.startup_health_pending = startup_pending;
    if let Some(path) = import {
        let record = import_model_record(&path)?;
        app.store.save_model(&record)?;
        println!("AETHERAI_MODEL_IMPORTED={}", record.descriptor.id);
        return Ok(());
    }
    if let Some(path) = import_chatgpt {
        let summary = app.import_chatgpt_archive_now(&path)?;
        println!(
            "AETHERAI_CHATGPT_IMPORT=PASS projects={} conversations={} messages={} attachments={} checkpoints={} fingerprint={}",
            summary.project_count,
            summary.conversation_count,
            summary.message_count,
            summary.attachment_count,
            summary.checkpoint_count,
            summary.fingerprint,
        );
        return Ok(());
    }
    if diagnostic {
        app.diagnostic();
        #[cfg(target_os = "linux")]
        native_ui::diagnostic();
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        match native_ui::run(app) {
            Ok(()) => Ok(()),
            Err(error) => {
                if startup_pending {
                    if let Some(manager) = lifecycle.as_ref() {
                        if recover_pending_startup(
                            manager,
                            &format!("native renderer startup failed: {error}"),
                        )? {
                            return Ok(());
                        }
                    }
                }
                Err(error)
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let mut app = app;
        return platform::run(&mut app);
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Err("unsupported AetherAI desktop platform".into())
    }
}
fn default_data_dir() -> PathBuf {
    let sys = aether_system::current_system_integration();
    sys.data_dir()
        .unwrap_or_else(|_| Path::new(".aetherai").to_path_buf())
}

#[cfg(test)]
mod v036_native_chatgpt_library_tests {
    #[test]
    fn v036_chatgpt_library_has_no_browser_runtime() {
        let main = include_str!("main.rs");
        let app = include_str!("app.rs");

        assert!(
            !main
                .lines()
                .map(str::trim)
                .any(|line| line == concat!("mod ", "chatgpt_webview;"))
        );
        assert!(!app.contains("open_official_url("));
        assert!(!app.contains("chatgpt.com"));
        assert!(!app.contains("privacy.openai.com"));
        assert!(!app.contains("latest_chatgpt_export_since"));
        assert!(app.contains("\"Import ChatGPT Library\""));
        assert!(app.contains("\"Open Imported Projects\""));
        assert!(app.contains("\"Re-import ChatGPT Export\""));
        assert!(app.contains("import_chatgpt_export_all"));
    }
}
