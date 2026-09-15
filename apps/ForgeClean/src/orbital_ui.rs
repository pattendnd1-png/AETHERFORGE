use eframe::egui;
use crate::orbital::{local_storage_summary, read_status};
use crate::orbital_monitor_model::{
    NetworkTotals, SyncProgress, read_network_totals, read_sync_progress,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const HISTORY_POINTS: usize = 120;
const REFRESH: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Default)]
struct TransferPoint {
    rx_bps: f64,
    tx_bps: f64,
}

#[derive(Clone, Default)]
struct Snapshot {
    mode: String,
    result: String,
    note: String,
    updated_at: String,
    local_head: String,
    remote_head: String,
    queue_ahead: usize,
    queue_behind: usize,
    dirty_count: usize,
    release_updated_at: String,
    orbital_timer: String,
    last_full_success_epoch: u64,
    repo: String,
    release_tag: String,
    indexed_artifacts: usize,
    externalized_large_files: usize,
    restore_helper_present: bool,
    progress: SyncProgress,
    rx_bps: f64,
    tx_bps: f64,
}

struct Cache {
    at: Instant,
    net_at: Instant,
    net: NetworkTotals,
    snapshot: Snapshot,
    history: VecDeque<TransferPoint>,
}

impl Cache {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            at: now - REFRESH,
            net_at: now,
            net: read_network_totals(),
            snapshot: Snapshot::default(),
            history: VecDeque::with_capacity(HISTORY_POINTS),
        }
    }
}

#[derive(Clone)]
struct View {
    snapshot: Snapshot,
    history: VecDeque<TransferPoint>,
}

static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
fn parse_usize(value: &str) -> usize {
    value.parse().unwrap_or(0)
}
fn parse_u64(value: &str) -> u64 {
    value.parse().unwrap_or(0)
}

fn load_snapshot(
    previous: &Snapshot,
    old_net: NetworkTotals,
    old_at: Instant,
) -> (Snapshot, NetworkTotals, Instant) {
    let Some(home) = home() else {
        return (previous.clone(), old_net, old_at);
    };
    let status = read_status(&home).ok();
    let storage = local_storage_summary(&home);
    let now = Instant::now();
    let current_net = read_network_totals();
    let secs = now.duration_since(old_at).as_secs_f64().max(0.001);
    let rx_bps = current_net.rx_bytes.saturating_sub(old_net.rx_bytes) as f64 / secs;
    let tx_bps = current_net.tx_bytes.saturating_sub(old_net.tx_bytes) as f64 / secs;
    let get = |key: &str| {
        status
            .as_ref()
            .map(|s| s.get(key).to_string())
            .unwrap_or_default()
    };
    let snapshot = Snapshot {
        mode: get("mode"),
        result: get("result"),
        note: get("note"),
        updated_at: get("updated_at"),
        local_head: get("local_head"),
        remote_head: get("remote_main"),
        queue_ahead: parse_usize(&get("queue_ahead")),
        queue_behind: parse_usize(&get("queue_behind")),
        dirty_count: parse_usize(&get("dirty_count")),
        release_updated_at: get("release_updated_at"),
        orbital_timer: get("orbital_timer"),
        last_full_success_epoch: parse_u64(&get("last_full_success_epoch")),
        repo: storage.repo,
        release_tag: storage.release_tag,
        indexed_artifacts: storage.indexed_artifacts,
        externalized_large_files: storage.externalized_large_files,
        restore_helper_present: storage.restore_helper_present,
        progress: read_sync_progress(&home),
        rx_bps,
        tx_bps,
    };
    (snapshot, current_net, now)
}

fn view(force: bool) -> View {
    let cache = CACHE.get_or_init(|| Mutex::new(Cache::new()));
    let mut guard = cache.lock().unwrap_or_else(|p| p.into_inner());
    if force || guard.at.elapsed() >= REFRESH {
        let (snapshot, net, net_at) = load_snapshot(&guard.snapshot, guard.net, guard.net_at);
        guard.snapshot = snapshot;
        guard.net = net;
        guard.net_at = net_at;
        let point = TransferPoint {
            rx_bps: guard.snapshot.rx_bps,
            tx_bps: guard.snapshot.tx_bps,
        };
        guard.history.push_back(point);
        while guard.history.len() > HISTORY_POINTS {
            guard.history.pop_front();
        }
        guard.at = Instant::now();
    }
    View {
        snapshot: guard.snapshot.clone(),
        history: guard.history.clone(),
    }
}

fn launch(args: &'static [&'static str]) {
    std::thread::spawn(move || {
        let _ = Command::new("forgeclean-system").args(args).status();
    });
}
fn open_url(url: &'static str) {
    std::thread::spawn(move || {
        let _ = Command::new("xdg-open").arg(url).status();
    });
}
fn short_sha(value: &str) -> &str {
    if value.len() > 12 {
        &value[..12]
    } else {
        value
    }
}

fn format_rate(bytes_per_sec: f64) -> String {
    const K: f64 = 1024.0;
    if bytes_per_sec >= K * K * K {
        format!("{:.1} GiB/s", bytes_per_sec / (K * K * K))
    } else if bytes_per_sec >= K * K {
        format!("{:.1} MiB/s", bytes_per_sec / (K * K))
    } else if bytes_per_sec >= K {
        format!("{:.1} KiB/s", bytes_per_sec / K)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

fn next_full_text(last: u64) -> String {
    if last == 0 {
        return "due".to_string();
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let elapsed = now.saturating_sub(last);
    if elapsed >= 300 {
        "due".to_string()
    } else {
        let remain = 300 - elapsed;
        format!("{:02}:{:02}", remain / 60, remain % 60)
    }
}

fn draw_network_graph(ui: &mut egui::Ui, history: &VecDeque<TransferPoint>) {
    let desired = egui::vec2(ui.available_width().max(240.0), 150.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let bg = egui::Color32::from_rgba_unmultiplied(18, 8, 30, 235);
    let grid = egui::Color32::from_rgba_unmultiplied(104, 62, 132, 55);
    let upload = egui::Color32::from_rgb(198, 108, 255);
    let restore = egui::Color32::from_rgb(238, 205, 255);
    painter.rect_filled(rect, 12.0, bg);
    for i in 1..4 {
        let y = egui::lerp(rect.top()..=rect.bottom(), i as f32 / 4.0);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, grid),
        );
    }
    if history.len() < 2 {
        return;
    }
    let peak = history
        .iter()
        .fold(1.0_f64, |m, p| m.max(p.rx_bps).max(p.tx_bps));
    let n = history.len();
    let point = |idx: usize, value: f64| {
        let x = rect.left() + rect.width() * (idx as f32 / (n.saturating_sub(1).max(1)) as f32);
        let normalized = (value / peak).clamp(0.0, 1.0) as f32;
        let y = rect.bottom() - rect.height() * normalized;
        egui::pos2(x, y)
    };
    let mut last_tx = None;
    let mut last_rx = None;
    for (idx, sample) in history.iter().enumerate() {
        let tx = point(idx, sample.tx_bps);
        let rx = point(idx, sample.rx_bps);
        if let Some(prev) = last_tx {
            painter.line_segment([prev, tx], egui::Stroke::new(2.0, upload));
        }
        if let Some(prev) = last_rx {
            painter.line_segment([prev, rx], egui::Stroke::new(1.6, restore));
        }
        last_tx = Some(tx);
        last_rx = Some(rx);
    }
}

fn status_word(s: &Snapshot) -> &'static str {
    if s.orbital_timer == "inactive" {
        "PAUSED"
    } else if s.progress.active {
        "BUSY"
    } else if s.queue_ahead > 0 || s.queue_behind > 0 {
        "QUEUED"
    } else if s.result == "PASS" {
        "HEALTHY"
    } else if s.result.is_empty() {
        "UNKNOWN"
    } else {
        "ATTENTION"
    }
}

pub fn show(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let current = view(false);
    let s = &current.snapshot;
    ui.horizontal(|ui| {
        ui.heading("Orbital Sync Monitor");
        ui.separator();
        ui.label(
            egui::RichText::new(status_word(s))
                .strong()
                .color(egui::Color32::from_rgb(222, 178, 255)),
        );
        ui.label(egui::RichText::new("● LIVE").color(egui::Color32::from_rgb(198, 108, 255)));
    });
    ui.label("NetworkCard-style live history — GitHub external storage");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("↑ Upload  {}", format_rate(s.tx_bps)))
                .color(egui::Color32::from_rgb(198, 108, 255)),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!("↓ Restore  {}", format_rate(s.rx_bps)))
                .color(egui::Color32::from_rgb(238, 205, 255)),
        );
    });
    draw_network_graph(ui, &current.history);
    ui.small("Live host network counters; ForgeClean does not invoke git/gh/scans from the 1-second graph loop.");
    ui.separator();

    ui.heading("Current Operation");
    let phase = if s.progress.active && !s.progress.phase.is_empty() {
        s.progress.phase.as_str()
    } else if s.result == "PASS" {
        "IDLE"
    } else {
        "UNKNOWN"
    };
    egui::Grid::new("forgeclean_orbital_progress_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Phase");
            ui.monospace(phase);
            ui.end_row();
            ui.label("Project");
            ui.label(if s.progress.project.is_empty() {
                "—"
            } else {
                &s.progress.project
            });
            ui.end_row();
            ui.label("Current file");
            ui.label(if s.progress.current_file.is_empty() {
                "—"
            } else {
                &s.progress.current_file
            });
            ui.end_row();
            ui.label("Assets");
            ui.label(if s.progress.total > 0 {
                format!("{} / {}", s.progress.completed, s.progress.total)
            } else {
                "—".to_string()
            });
            ui.end_row();
            ui.label("Mode");
            ui.label(if s.mode.is_empty() {
                "unknown"
            } else {
                &s.mode
            });
            ui.end_row();
            ui.label("Result");
            ui.label(if s.result.is_empty() {
                "unknown"
            } else {
                &s.result
            });
            ui.end_row();
            ui.label("Updated");
            ui.label(if s.updated_at.is_empty() {
                "not yet"
            } else {
                &s.updated_at
            });
            ui.end_row();
            ui.label("Next fast orbit");
            ui.label("30s cadence");
            ui.end_row();
            ui.label("Next full orbit");
            ui.label(next_full_text(s.last_full_success_epoch));
            ui.end_row();
        });

    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("Sync Now").clicked() {
            launch(&["orbital", "sync"]);
        }
        if ui.button("Full Scan + Upload").clicked() {
            launch(&["migration", "upload"]);
        }
        if ui.button("Retry Queue").clicked() {
            launch(&["orbital", "retry"]);
        }
        if ui.button("Refresh").clicked() {
            let _ = view(true);
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button("Pause Orbit").clicked() {
            launch(&["orbital", "pause"]);
        }
        if ui.button("Resume Orbit").clicked() {
            launch(&["orbital", "resume"]);
        }
        if ui.button("Verify Remote Storage").clicked() {
            launch(&["migration", "verify"]);
        }
    });

    ui.separator();
    ui.heading("GitHub External Storage");
    egui::Grid::new("forgeclean_storage_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Repository");
            ui.label(&s.repo);
            ui.end_row();
            ui.label("Release vault");
            ui.label(&s.release_tag);
            ui.end_row();
            ui.label("Release updated");
            ui.label(if s.release_updated_at.is_empty() {
                "unknown"
            } else {
                &s.release_updated_at
            });
            ui.end_row();
            ui.label("Indexed artifacts");
            ui.label(s.indexed_artifacts.to_string());
            ui.end_row();
            ui.label("Large external assets");
            ui.label(s.externalized_large_files.to_string());
            ui.end_row();
            ui.label("Queue ahead / behind");
            ui.label(format!("{} / {}", s.queue_ahead, s.queue_behind));
            ui.end_row();
            ui.label("Dirty files");
            ui.label(s.dirty_count.to_string());
            ui.end_row();
            ui.label("Local HEAD");
            ui.monospace(short_sha(&s.local_head));
            ui.end_row();
            ui.label("Remote HEAD");
            ui.monospace(short_sha(&s.remote_head));
            ui.end_row();
            ui.label("Restore helper");
            ui.label(if s.restore_helper_present {
                "ready"
            } else {
                "not present"
            });
            ui.end_row();
        });
    ui.horizontal_wrapped(|ui| {
        if ui.button("Migrate Now").clicked() {
            launch(&["migration", "sync"]);
        }
        if ui.button("Restore From GitHub").clicked() {
            launch(&["migration", "restore"]);
        }
        if ui.button("Open Repository").clicked() {
            open_url("https://github.com/pattendnd1-png/AETHERFORGE");
        }
        if ui.button("Open WIP Releases").clicked() {
            open_url("https://github.com/pattendnd1-png/AETHERFORGE/releases/tag/wip-post-reset");
        }
    });

    ui.separator();
    ui.heading("Activity");
    ui.monospace(format!(
        "{}\nphase={}\nnote={}\nqueue={}\n",
        s.updated_at,
        phase,
        if s.note.is_empty() { "—" } else { &s.note },
        s.queue_ahead + s.queue_behind
    ));
    ui.small("Deep-purple DragonGlass monitor. No pie charts. The orbital worker remains the only sync authority.");
    ctx.request_repaint_after(REFRESH);
}
