use std::time::{Duration, Instant};

use aether_telemetry::{
    CpuTimes, FrameRateTracker, StreamTelemetrySnapshot, cpu_percent_between, loss_basis_points,
    parse_meminfo, parse_net_dev, parse_proc_stat,
};

#[test]
fn parses_linux_cpu_delta() {
    let previous = CpuTimes {
        total: 1_000,
        idle: 700,
    };
    let current = CpuTimes {
        total: 1_200,
        idle: 800,
    };
    assert_eq!(cpu_percent_between(previous, current), Some(50));
    assert_eq!(
        parse_proc_stat("cpu  10 2 3 40 5 1 1 0 99 0\n"),
        Some(CpuTimes {
            total: 62,
            idle: 45
        })
    );
}

#[test]
fn parses_memory_and_network_without_loopback() {
    let memory = parse_meminfo(
        "MemTotal: 1000 kB\nMemAvailable: 400 kB\nSwapTotal: 500 kB\nSwapFree: 300 kB\n",
    )
    .unwrap();
    assert_eq!(memory.used_bytes(), 600 * 1024);
    assert_eq!(memory.swap_used_bytes(), 200 * 1024);

    let net = parse_net_dev("Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n lo: 100 0 0 0 0 0 0 0 100 0 0 0 0 0 0 0\n eth0: 2000 0 0 0 0 0 0 0 3000 0 0 0 0 0 0 0\n").unwrap();
    assert_eq!(net.received_bytes, 2_000);
    assert_eq!(net.transmitted_bytes, 3_000);
}

#[test]
fn frame_loss_metrics_keep_render_output_and_network_separate() {
    let stats = StreamTelemetrySnapshot {
        render_skipped_frames: 10,
        output_skipped_frames: 5,
        network_dropped_frames: 2,
        total_frames: 10_000,
        ..StreamTelemetrySnapshot::default()
    };
    assert_eq!(stats.total_lost_frames(), 17);
    assert_eq!(stats.render_loss_basis_points(), Some(10));
    assert_eq!(stats.output_loss_basis_points(), Some(5));
    assert_eq!(stats.network_loss_basis_points(), Some(2));
    assert_eq!(loss_basis_points(25, 10_000), Some(25));
}

#[test]
fn frame_rate_tracker_reports_milli_fps() {
    let start = Instant::now();
    let mut tracker = FrameRateTracker::new(start);
    for frame in 1_u64..50 {
        assert_eq!(
            tracker.record_frame(start + Duration::from_millis(frame * 10)),
            None
        );
    }
    let fps = tracker
        .record_frame(start + Duration::from_millis(500))
        .unwrap();
    assert_eq!(fps, 100_000);
}
