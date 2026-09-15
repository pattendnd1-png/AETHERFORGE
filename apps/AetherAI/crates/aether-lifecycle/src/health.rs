use crate::LifecycleError;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub fn run_health_diagnostic(root: &Path, binary: &str) -> Result<(), LifecycleError> {
    let executable = root.join(binary);
    let mut child = Command::new(&executable)
        .arg("--diagnostic")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LifecycleError::Health(format!("failed to start diagnostic: {error}")))?;

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child
            .try_wait()
            .map_err(|error| LifecycleError::Health(error.to_string()))?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| LifecycleError::Health(error.to_string()))?;
            if !output.status.success() {
                return Err(LifecycleError::Health(format!(
                    "diagnostic exited with {}",
                    output.status
                )));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            for required in [
                "AETHERAI_DESKTOP=LIVE",
                "AETHERAI_NORMAL_USER_TERMINAL_REQUIRED=NO",
            ] {
                if !stdout.lines().any(|line| line.trim() == required) {
                    return Err(LifecycleError::Health(format!(
                        "diagnostic contract missing: {required}"
                    )));
                }
            }
            return Ok(());
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(LifecycleError::Health(
                "diagnostic exceeded 10 second health timeout".into(),
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}
