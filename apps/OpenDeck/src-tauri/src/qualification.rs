use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

const MAX_SAMPLES: usize = 512;
static RUNTIME_SAMPLES: OnceLock<Mutex<BTreeMap<String, Vec<f64>>>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationContext {
    pub enabled: bool,
    pub phase: String,
}

pub fn enabled() -> bool {
    std::env::var("OPENDECK_V210_QUALIFICATION").is_ok_and(|value| value == "1")
}

pub fn phase() -> String {
    std::env::var("OPENDECK_V210_QUALIFICATION_PHASE")
        .ok()
        .filter(|value| value == "visual" || value == "performance")
        .unwrap_or_else(|| "visual".into())
}

pub fn output_dir() -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os("OPENDECK_QUALIFICATION_DIR") {
        return Ok(PathBuf::from(value));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join("Downloads"))
}

pub fn workspace_benchmark_path() -> Result<PathBuf, String> {
    Ok(output_dir()?.join(".OpenDeck-v2.0.10-workspace-benchmark.json"))
}

fn atomic_json(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "qualification output has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temp = parent.join(format!(".{}.{}.tmp", path.file_name().unwrap_or_default().to_string_lossy(), Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = File::create(&temp).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temp, path).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn record_runtime_sample(name: &str, duration_ms: f64) {
    if !enabled() || !duration_ms.is_finite() || duration_ms < 0.0 {
        return;
    }
    let samples = RUNTIME_SAMPLES.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Ok(mut map) = samples.lock() {
        let values = map.entry(name.to_string()).or_default();
        values.push(duration_ms);
        if values.len() > MAX_SAMPLES {
            let remove = values.len() - MAX_SAMPLES;
            values.drain(0..remove);
        }
    }
}

pub fn percentile_nearest_rank(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let bounded = p.clamp(0.0, 1.0);
    let rank = ((bounded * sorted.len() as f64).ceil() as usize).max(1);
    sorted[(rank - 1).min(sorted.len() - 1)]
}

fn runtime_snapshot() -> Value {
    let Some(samples) = RUNTIME_SAMPLES.get() else { return json!({}); };
    let Ok(map) = samples.lock() else { return json!({}); };
    let mut output = Map::new();
    for (name, values) in map.iter() {
        if values.is_empty() { continue; }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        output.insert(name.clone(), json!({
            "count": values.len(),
            "min": values.iter().copied().fold(f64::INFINITY, f64::min),
            "max": values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            "mean": mean,
            "p50": percentile_nearest_rank(values, 0.50),
            "p95": percentile_nearest_rank(values, 0.95),
            "p99": percentile_nearest_rank(values, 0.99),
        }));
    }
    Value::Object(output)
}

#[tauri::command]
pub fn qualification_context() -> QualificationContext {
    QualificationContext { enabled: enabled(), phase: phase() }
}

#[tauri::command]
pub fn qualification_record_ui_metrics(payload: Value) -> Result<(), String> {
    if !enabled() {
        return Err("OpenDeck v2.0.10 qualification mode is not enabled".into());
    }
    let kind = payload.get("kind").and_then(Value::as_str).unwrap_or("error");
    let mut body = payload.get("payload").cloned().unwrap_or_else(|| json!({}));
    if kind == "performance" {
        if let Some(object) = body.as_object_mut() {
            object.insert("rust".into(), runtime_snapshot());
        }
    }
    let name = match kind {
        "visual" => "OpenDeck-v2.0.10-VISUAL-METRICS.json",
        "performance" => "OpenDeck-v2.0.10-PERFORMANCE-METRICS.json",
        _ => "OpenDeck-v2.0.10-QUALIFICATION-ERROR.json",
    };
    atomic_json(&output_dir()?.join(name), &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_percentiles_are_stable() {
        let values: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile_nearest_rank(&values, 0.95), 95.0);
        assert_eq!(percentile_nearest_rank(&values, 0.99), 99.0);
    }
}
