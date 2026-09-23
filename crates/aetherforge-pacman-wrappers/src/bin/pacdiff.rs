//! AetherForge pacdiff replacement
//! Finds and manages .pacnew/.pacsave files

use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Find and manage .pacnew/.pacsave files
#[derive(Parser, Debug)]
#[command(name = "pacdiff-rs")]
#[command(author = "AetherForge Team")]
#[command(version = "1.0.0")]
#[command(about = "Intelligent .pacnew/.pacsave manager")]
struct Args {
    /// Show differences between original and .pacnew
    #[arg(short, long)]
    diff: bool,

    /// Automatically merge all .pacnew files (original kept as .pacsave)
    #[arg(short, long)]
    merge_all: bool,

    /// Show summary only
    #[arg(short, long)]
    summary: bool,

    /// Search root (default: /)
    #[arg(short, long)]
    root: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct ConfigFile {
    original: PathBuf,
    pacnew: PathBuf,
}

fn find_config_files(root: &Path) -> Result<Vec<ConfigFile>> {
    let mut configs = Vec::new();

    for entry in WalkDir::new(root).into_iter()
        .filter_entry(|e| !e.path().starts_with("/proc"))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        if path.extension().and_then(|e| e.to_str()) == Some("pacnew") {
            let original = path.with_extension("");
            
            if original.exists() {
                configs.push(ConfigFile {
                    original,
                    pacnew: path.to_path_buf(),
                });
            }
        }
    }
    
    Ok(configs)
}

fn show_diff(original: &Path, pacnew: &Path) -> Result<()> {
    println!("\n--- {} ---", original.display());
    println!("+++ {} +++", pacnew.display());
    println!("─────────────────────────────");

    let output = Command::new("diff")
        .args(["-u", "--color=auto"])
        .arg(original)
        .arg(pacnew)
        .output()?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let root = args.root.unwrap_or_else(|| PathBuf::from("/"));
    
    if args.verbose {
        eprintln!("Scanning from: {}", root.display());
    }

    let configs = find_config_files(&root)?;
    
    if configs.is_empty() {
        println!("No .pacnew files found.");
        return Ok(());
    }

    println!("\n=== AetherForge Pacdiff Scanner ===");
    println!("Found {} .pacnew file(s):\n", configs.len());

    for (i, config) in configs.iter().enumerate() {
        println!("{}. {}", i + 1, config.original.display());
        println!("   .pacnew: {}", config.pacnew.display());
    }

    if args.summary {
        return Ok(());
    }

    if args.diff {
        for config in &configs {
            if let Err(e) = show_diff(&config.original, &config.pacnew) {
                eprintln!("Failed to diff {:?}: {}", config.pacnew, e);
            }
        }
    }

    if args.merge_all {
        println!("\nMerge all .pacnew files? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout())?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if input.trim().eq_ignore_ascii_case("y") {
            for config in &configs {
                // Create backup
                let backup_path = PathBuf::from(format!("{}.pacsave", config.original.display()));
                if !backup_path.exists() {
                    fs::copy(&config.original, &backup_path)?;
                    println!("Backup created: {}", backup_path.display());
                }
                // Copy new over original
                fs::copy(&config.pacnew, &config.original)?;
                println!("✓ Merged: {}", config.original.display());
            }
        }
    }

    println!("\nTo review changes interactively:");
    println!("  pacdiff-rs --diff");
    println!();
    println!("To auto-merge all (with backups):");
    println!("  sudo pacdiff-rs --merge-all");

    Ok(())
}
