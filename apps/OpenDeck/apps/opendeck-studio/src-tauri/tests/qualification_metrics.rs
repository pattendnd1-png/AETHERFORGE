use opendeck_studio_lib::qualification::percentile_nearest_rank;

#[test]
fn qualification_metrics_use_nearest_rank_percentiles() {
    let samples: Vec<f64> = (1..=100).map(f64::from).collect();
    assert_eq!(percentile_nearest_rank(&samples, 0.95), 95.0);
    assert_eq!(percentile_nearest_rank(&samples, 0.99), 99.0);
}
