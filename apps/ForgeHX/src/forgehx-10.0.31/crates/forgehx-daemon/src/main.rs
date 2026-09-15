use forgehx_daemon::{run_server, DaemonState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let mut state = DaemonState::system();
    if let Err(error) = state.refresh_devices() {
        eprintln!("ForgeHX initial device scan failed: {error}");
    }
    run_server(state).await?;
    Ok(())
}
