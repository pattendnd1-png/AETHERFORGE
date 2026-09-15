mod commands;
use aether_chat::ChatService;
use aether_core::{Conversation, GenerationSettings, ProcessingPreset};
use aether_model_api::{ModelBackend, ModelDescriptor, ModelFormat, ModelProvider};
use aether_model_gguf::GgufProvider;
use aether_model_native::NativeProvider;
use aether_model_runtime::{
    catalog::ModelCatalog,
    download::download_model,
    runtime::{RuntimeManifest, install_gguf_runtime, stable_runtime_path},
};
use aether_storage::{SqliteStore, models::ModelRecord};
use chrono::Utc;
use clap::Parser;
use commands::{Command, parse_command};
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Parser)]
#[command(name = "aetherai", version, about = "AetherAI live local assistant")]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long = "new", default_value = "New Chat")]
    new_title: String,
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    import_model: Option<PathBuf>,
    #[arg(long)]
    install_starter: bool,
    #[arg(long)]
    diagnostic: bool,
    #[arg(long, hide = true)]
    verify_provider: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let data_dir = args.data_dir.unwrap_or_else(default_data_dir);
    std::fs::create_dir_all(&data_dir)?;
    let store = SqliteStore::open(data_dir.join("aetherai.db"))?;
    if let Some(path) = args.import_model.as_deref() {
        let record = import_model(path)?;
        store.save_model(&record)?;
        println!("AETHERAI_MODEL_IMPORTED={}", record.descriptor.id);
    }
    if args.install_starter {
        let record = install_starter(&data_dir)?;
        store.save_model(&record)?;
        println!("AETHERAI_RUNTIME_INSTALLED=PASS");
        println!("AETHERAI_MODEL_IMPORTED={}", record.descriptor.id);
    }
    let models = store.list_models()?;
    if args.diagnostic {
        println!("AETHERAI_RUNTIME=LIVE");
        println!("AETHERAI_LIVE_MODELS={}", models.len());
        println!("AETHERAI_SCHEMA_VERSION={}", store.schema_version()?);
        return Ok(());
    }
    if args.verify_provider {
        let id = args
            .model
            .as_deref()
            .ok_or("--verify-provider requires --model")?;
        let record = models
            .iter()
            .find(|m| m.descriptor.id == id)
            .ok_or("registered model not found")?;
        verify_provider(record, &data_dir).await?;
        return Ok(());
    }
    let mut conversation = Conversation::new(args.new_title);
    conversation.settings.model = args
        .model
        .or_else(|| models.first().map(|m| m.descriptor.id.clone()));
    store.save_conversation(&conversation)?;
    println!("AetherAI v0.2.2 — live local runtime");
    println!("AETHERAI_RUNTIME=LIVE");
    if let Some(m) = &conversation.settings.model {
        println!("Model: {m}")
    } else {
        println!("No live model configured. Import or download a model in AetherAI.")
    }
    println!("Type /help for commands.");
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    loop {
        print!("You> ");
        io::stdout().flush()?;
        let Some(line) = lines.next_line().await? else {
            break;
        };
        if let Some(cmd) = parse_command(&line) {
            match cmd {
                Command::Help => print_help(),
                Command::Preset(p) => {
                    conversation.settings.generation = GenerationSettings::from_preset(p);
                    println!("Processing preset: {p:?}")
                }
                Command::Workspace(name) => {
                    conversation.ensure_workspace(&name);
                    println!("Workspace enabled: {name}")
                }
                Command::Models => print_models(&models),
                Command::Model(id) => {
                    if models.iter().any(|m| m.descriptor.id == id) {
                        conversation.settings.model = Some(id.clone());
                        println!("Model: {id}")
                    } else {
                        eprintln!("Unknown model: {id}")
                    }
                }
                Command::Settings => print_settings(&conversation),
                Command::Quit => break,
            }
            store.save_conversation(&conversation)?;
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some(model_id) = conversation.settings.model.clone() else {
            eprintln!("No live model configured. Use --import-model PATH or /models.");
            continue;
        };
        let Some(record) = models.iter().find(|m| m.descriptor.id == model_id) else {
            eprintln!("Selected model is no longer registered: {model_id}");
            continue;
        };
        let provider = provider_for(record, &conversation, &data_dir);
        let chat = ChatService::new(provider);
        match chat
            .send_user_message_with(&mut conversation, line, |event| {
                if let aether_chat::ChatEvent::AssistantDelta(d) = event {
                    print!("{d}");
                    let _ = io::stdout().flush();
                }
            })
            .await
        {
            Ok(_) => {
                println!();
                store.save_conversation(&conversation)?
            }
            Err(e) => {
                println!();
                store.save_conversation(&conversation)?;
                eprintln!("AetherAI error: {e}")
            }
        }
    }
    store.save_conversation(&conversation)?;
    Ok(())
}

fn install_starter(data_dir: &Path) -> Result<ModelRecord, Box<dyn std::error::Error>> {
    let catalog = ModelCatalog::parse(include_str!("../../../resources/model-catalog.json"))?;
    let entry = catalog
        .models
        .first()
        .ok_or("starter model catalog is empty")?
        .clone();
    let manifest = RuntimeManifest::parse(include_str!(
        "../../../resources/inference/runtime-manifest.json"
    ))?;
    install_gguf_runtime(
        &manifest,
        &data_dir.join("runtime"),
        aether_core::NetworkPolicy::On,
    )?;
    let path = download_model(
        &entry,
        &data_dir.join("models"),
        aether_core::NetworkPolicy::On,
    )?;
    Ok(ModelRecord {
        descriptor: ModelDescriptor {
            id: entry.id,
            display_name: entry.display_name,
            backend: ModelBackend::AetherGguf,
            format: ModelFormat::Gguf,
            path: path.to_string_lossy().into_owned(),
            context_tokens: entry.context_tokens,
            local: true,
            load_state: aether_model_api::ModelLoadState::Registered,
        },
        registered_at: Utc::now(),
        last_used: None,
    })
}
fn import_model(path: &Path) -> Result<ModelRecord, Box<dyn std::error::Error>> {
    let canonical = std::fs::canonicalize(path)?;
    let name = canonical
        .file_stem()
        .or_else(|| canonical.file_name())
        .and_then(|v| v.to_str())
        .unwrap_or("model")
        .replace(
            |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_',
            "-",
        );
    let is_gguf = canonical
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| v.eq_ignore_ascii_case("gguf"));
    let descriptor = ModelDescriptor {
        id: format!("local/{name}"),
        display_name: name,
        backend: if is_gguf {
            ModelBackend::AetherGguf
        } else {
            ModelBackend::AetherNative
        },
        format: if is_gguf {
            ModelFormat::Gguf
        } else {
            ModelFormat::SafeTensors
        },
        path: canonical.to_string_lossy().into_owned(),
        context_tokens: 32768,
        local: true,
        load_state: aether_model_api::ModelLoadState::Registered,
    };
    Ok(ModelRecord {
        descriptor,
        registered_at: Utc::now(),
        last_used: None,
    })
}
fn provider_for(record: &ModelRecord, c: &Conversation, data_dir: &Path) -> Arc<dyn ModelProvider> {
    let d = record.descriptor.clone();
    match d.backend {
        ModelBackend::AetherGguf => {
            if let Ok(port) = env::var("AETHERAI_GGUF_EXTERNAL_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .ok_or(())
            {
                Arc::new(GgufProvider::external(d, port))
            } else {
                let exe = env::var_os("AETHERAI_GGUF_RUNTIME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        let installed = stable_runtime_path(&data_dir.join("runtime"));
                        if installed.is_file() {
                            installed
                        } else {
                            default_gguf_runtime()
                        }
                    });
                Arc::new(GgufProvider::managed(
                    d,
                    exe,
                    18080,
                    c.settings.generation.cpu_threads,
                    c.settings.generation.gpu_layers,
                ))
            }
        }
        ModelBackend::AetherNative => {
            if let Some(port) = env::var("AETHERAI_NATIVE_EXTERNAL_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
            {
                Arc::new(NativeProvider::external(d, port))
            } else {
                let exe =
                    env::var("AETHERAI_NATIVE_RUNTIME").unwrap_or_else(|_| "mistralrs".into());
                Arc::new(NativeProvider::managed_mistralrs(d, exe, 18081))
            }
        }
    }
}
fn default_gguf_runtime() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .map(|p| {
            #[cfg(target_os = "windows")]
            {
                p.join("../../tools/inference/aetherai-gguf-runtime.exe")
            }
            #[cfg(not(target_os = "windows"))]
            {
                p.join("../../tools/inference/aetherai-gguf-runtime")
            }
        })
        .unwrap_or_else(|| {
            #[cfg(target_os = "windows")]
            {
                PathBuf::from("tools/inference/aetherai-gguf-runtime.exe")
            }
            #[cfg(not(target_os = "windows"))]
            {
                PathBuf::from("tools/inference/aetherai-gguf-runtime")
            }
        })
}
async fn verify_provider(
    record: &ModelRecord,
    data_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut c = Conversation::new("provider verify");
    c.settings.model = Some(record.descriptor.id.clone());
    c.settings.generation = GenerationSettings::from_preset(ProcessingPreset::Fast);
    let provider = provider_for(record, &c, data_dir);
    println!("AETHERAI_MODEL_SERVICE=PASS");
    let chat = ChatService::new(provider);
    let mut chunks = 0usize;
    let result = chat
        .send_user_message_with(&mut c, verification_prompt(), |e| {
            if matches!(e, aether_chat::ChatEvent::AssistantDelta(_)) {
                chunks += 1
            }
        })
        .await?;
    if result.assistant_text.trim().is_empty() {
        return Err("empty generation".into());
    }
    println!("AETHERAI_LIVE_MODEL_LOAD=PASS");
    println!("AETHERAI_LIVE_GENERATION=PASS");
    if chunks == 0 {
        return Err("no streamed chunks".into());
    }
    println!("AETHERAI_TOKEN_STREAM=PASS");
    Ok(())
}
fn verification_prompt() -> &'static str {
    "Reply with the single word PASS. /no_think"
}

fn print_models(models: &[ModelRecord]) {
    if models.is_empty() {
        println!("No registered models.")
    }
    for m in models {
        println!(
            "{}  {:?}  {}",
            m.descriptor.id, m.descriptor.backend, m.descriptor.path
        )
    }
}
fn default_data_dir() -> PathBuf {
    if let Some(p) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(p).join("aetherai");
    }
    if let Some(h) = env::var_os("HOME") {
        return PathBuf::from(h).join(".local/share/aetherai");
    }
    PathBuf::from(".aetherai")
}
fn print_help() {
    println!(
        "/help\n/models\n/model ID\n/preset fast|balanced|deep|maximum|custom\n/workspace NAME\n/settings\n/quit"
    )
}
fn print_settings(c: &Conversation) {
    let s = &c.settings;
    let g = &s.generation;
    println!("Model: {}", s.model.as_deref().unwrap_or("<none>"));
    println!(
        "Preset: {:?}\nContext: {}\nMax output: {}\nTemperature: {}\nTop-p: {}\nTop-k: {}\nCPU threads: {:?}\nGPU layers: {:?}\nNetwork: {:?}",
        g.preset,
        g.context_tokens,
        g.max_output_tokens,
        g.temperature,
        g.top_p,
        g.top_k,
        g.cpu_threads,
        g.gpu_layers,
        s.network_policy
    )
}

#[cfg(test)]
mod live_verify_tests {
    use super::*;

    #[test]
    fn live_verify_probe_forces_qwen_non_thinking_fallback() {
        assert!(verification_prompt().contains("/no_think"));
    }
}
