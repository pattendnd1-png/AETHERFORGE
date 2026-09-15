use rustls::{ClientConfig, ClientConnection, RootCertStore, pki_types::ServerName};
use std::{
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::Arc,
    time::{Duration, Instant},
};

const PROBE_HOSTS: [&str; 2] = ["account.battle.net", "download.battle.net"];
const HTTPS_PORT: u16 = 443;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const IO_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkBridgeState {
    #[default]
    Unknown,
    Online,
    Degraded,
    Offline,
}

impl NetworkBridgeState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Checking",
            Self::Online => "Online",
            Self::Degraded => "Degraded",
            Self::Offline => "Offline",
        }
    }

    pub fn usable(self) -> bool {
        matches!(self, Self::Online | Self::Degraded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCheckKind {
    Dns,
    Tcp,
    Tls,
}

impl NetworkCheckKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dns => "DNS",
            Self::Tcp => "TCP/443",
            Self::Tls => "TLS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkCheck {
    pub host: String,
    pub kind: NetworkCheckKind,
    pub ok: bool,
    pub detail: String,
    pub latency_ms: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkBridgeReport {
    pub state: NetworkBridgeState,
    pub summary: String,
    pub checks: Vec<NetworkCheck>,
}

impl Default for NetworkBridgeReport {
    fn default() -> Self {
        Self {
            state: NetworkBridgeState::Unknown,
            summary: "Battle.net network bridge has not been checked yet.".into(),
            checks: Vec::new(),
        }
    }
}

impl NetworkBridgeReport {
    pub fn offline(summary: impl Into<String>) -> Self {
        Self {
            state: NetworkBridgeState::Offline,
            summary: summary.into(),
            checks: Vec::new(),
        }
    }

    pub fn repair_recommended(&self) -> bool {
        matches!(
            self.state,
            NetworkBridgeState::Degraded | NetworkBridgeState::Offline
        )
    }
}

pub fn network_probe_hosts() -> &'static [&'static str] {
    &PROBE_HOSTS
}

pub fn probe_network_bridge() -> NetworkBridgeReport {
    let root_store = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth(),
    );

    let mut checks = Vec::with_capacity(PROBE_HOSTS.len() * 3);
    for host in PROBE_HOSTS {
        probe_host(host, Arc::clone(&config), &mut checks);
    }
    classify_checks(checks)
}

fn probe_host(host: &str, config: Arc<ClientConfig>, checks: &mut Vec<NetworkCheck>) {
    let dns_started = Instant::now();
    let addresses = match (host, HTTPS_PORT).to_socket_addrs() {
        Ok(addresses) => addresses.collect::<Vec<_>>(),
        Err(error) => {
            checks.push(NetworkCheck {
                host: host.into(),
                kind: NetworkCheckKind::Dns,
                ok: false,
                detail: format!("DNS lookup failed: {error}"),
                latency_ms: Some(dns_started.elapsed().as_millis()),
            });
            push_skipped_transport_checks(host, "DNS resolution failed", checks);
            return;
        }
    };
    if addresses.is_empty() {
        checks.push(NetworkCheck {
            host: host.into(),
            kind: NetworkCheckKind::Dns,
            ok: false,
            detail: "DNS lookup returned no addresses.".into(),
            latency_ms: Some(dns_started.elapsed().as_millis()),
        });
        push_skipped_transport_checks(host, "DNS returned no addresses", checks);
        return;
    }
    checks.push(NetworkCheck {
        host: host.into(),
        kind: NetworkCheckKind::Dns,
        ok: true,
        detail: format!("Resolved {} address(es).", addresses.len()),
        latency_ms: Some(dns_started.elapsed().as_millis()),
    });

    let tcp_started = Instant::now();
    let mut stream = match connect_first(&addresses) {
        Ok(stream) => stream,
        Err(detail) => {
            checks.push(NetworkCheck {
                host: host.into(),
                kind: NetworkCheckKind::Tcp,
                ok: false,
                detail,
                latency_ms: Some(tcp_started.elapsed().as_millis()),
            });
            checks.push(NetworkCheck {
                host: host.into(),
                kind: NetworkCheckKind::Tls,
                ok: false,
                detail: "TLS check skipped because TCP/443 is unreachable.".into(),
                latency_ms: None,
            });
            return;
        }
    };
    checks.push(NetworkCheck {
        host: host.into(),
        kind: NetworkCheckKind::Tcp,
        ok: true,
        detail: "Outbound TCP/443 connected.".into(),
        latency_ms: Some(tcp_started.elapsed().as_millis()),
    });

    let tls_started = Instant::now();
    let (tls_ok, tls_detail) = match perform_tls_handshake(host, &mut stream, config) {
        Ok(detail) => (true, detail),
        Err(detail) => (false, detail),
    };
    checks.push(NetworkCheck {
        host: host.into(),
        kind: NetworkCheckKind::Tls,
        ok: tls_ok,
        detail: tls_detail,
        latency_ms: Some(tls_started.elapsed().as_millis()),
    });
}

fn connect_first(addresses: &[SocketAddr]) -> Result<TcpStream, String> {
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(address, CONNECT_TIMEOUT) {
            Ok(stream) => {
                if let Err(error) = stream.set_read_timeout(Some(IO_TIMEOUT)) {
                    return Err(format!("Could not configure network read timeout: {error}"));
                }
                if let Err(error) = stream.set_write_timeout(Some(IO_TIMEOUT)) {
                    return Err(format!(
                        "Could not configure network write timeout: {error}"
                    ));
                }
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.map_or_else(
        || "No resolved address could be connected.".into(),
        |error| format!("Outbound TCP/443 failed: {error}"),
    ))
}

fn perform_tls_handshake(
    host: &str,
    stream: &mut TcpStream,
    config: Arc<ClientConfig>,
) -> Result<String, String> {
    let server_name = ServerName::try_from(host.to_owned())
        .map_err(|error| format!("Invalid TLS server name: {error}"))?;
    let mut connection = ClientConnection::new(config, server_name)
        .map_err(|error| format!("Could not initialize TLS: {error}"))?;
    while connection.is_handshaking() {
        connection
            .complete_io(stream)
            .map_err(|error| format!("TLS handshake failed: {error}"))?;
    }
    Ok("Certificate-validated TLS handshake completed.".into())
}

fn push_skipped_transport_checks(host: &str, reason: &str, checks: &mut Vec<NetworkCheck>) {
    for kind in [NetworkCheckKind::Tcp, NetworkCheckKind::Tls] {
        checks.push(NetworkCheck {
            host: host.into(),
            kind,
            ok: false,
            detail: format!("{} check skipped: {reason}.", kind.label()),
            latency_ms: None,
        });
    }
}

fn classify_checks(checks: Vec<NetworkCheck>) -> NetworkBridgeReport {
    let expected_tls = PROBE_HOSTS.len();
    let tls_ok = checks
        .iter()
        .filter(|check| check.kind == NetworkCheckKind::Tls && check.ok)
        .count();
    let tcp_ok = checks
        .iter()
        .any(|check| check.kind == NetworkCheckKind::Tcp && check.ok);

    let (state, summary) = if tls_ok == expected_tls {
        (
            NetworkBridgeState::Online,
            "Battle.net DNS, TCP/443, and TLS connectivity are healthy.".into(),
        )
    } else if tls_ok > 0 || tcp_ok {
        (
            NetworkBridgeState::Degraded,
            "Battle.net network connectivity is only partially healthy; repair is recommended."
                .into(),
        )
    } else {
        (
            NetworkBridgeState::Offline,
            "Battle.net endpoints are unreachable from OpenSanctuary.".into(),
        )
    };

    NetworkBridgeReport {
        state,
        summary,
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(kind: NetworkCheckKind, ok: bool) -> NetworkCheck {
        NetworkCheck {
            host: "account.battle.net".into(),
            kind,
            ok,
            detail: String::new(),
            latency_ms: None,
        }
    }

    #[test]
    fn no_transport_success_is_offline() {
        let report = classify_checks(vec![
            check(NetworkCheckKind::Dns, false),
            check(NetworkCheckKind::Tcp, false),
            check(NetworkCheckKind::Tls, false),
        ]);
        assert_eq!(report.state, NetworkBridgeState::Offline);
    }

    #[test]
    fn partial_transport_success_is_degraded() {
        let report = classify_checks(vec![
            check(NetworkCheckKind::Dns, true),
            check(NetworkCheckKind::Tcp, true),
            check(NetworkCheckKind::Tls, false),
        ]);
        assert_eq!(report.state, NetworkBridgeState::Degraded);
    }
}
