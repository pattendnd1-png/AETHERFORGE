//! AetherForge paccache replacement
//! Cleans up pacman package cache intelligently

use anyhow::Result;
use clap::Parser;
use glob::glob;
use log::{info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Clean up old packages from the pacman cache
#[derive(Parser, Debug)]
#[command(name = "paccache-rs")]
#[command(author = "AetherForge Team")]
#[command(version = "1.0.0")]
#[command(about = "Smart package cache cleaner for AetherForge")]
struct Args {
    /// Number of versions to keep per package (default: 3)
    #[arg(short, long, default_value_t = 3)]
    keep: u32,

    /// Dry run - don't actually delete files
    #[arg(short, long)]
    dry_run: bool,

    /// Cache directory (default: /var/cache/pacman/pkg)
    #[arg(short, long)]
    cache_dir: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct PackageInfo {
    name: String,
    path: PathBuf,
    mtime: SystemTime,
    size: u64,
}

fn parse_pkg_name(path: &PathBuf) -> Option<String> {
    let filename = path.file_name()?.to_str()?;
    
    // Pattern: packagename-version-architecture.pkg.tar.zst
    let parts: Vec<&str> = filename.split('-').collect();
    if parts.len() < 4 {
        return None;
    }

    Some(parts[..parts.len()-3].join("-"))
}

fn collect_packages(cache_dir: &PathBuf) -> Result<Vec<PackageInfo>> {
    let mut packages = Vec::new();
    
    for entry in glob(&format!("{}/pkg/*.pkg.tar.*", cache_dir.display()))? {
        match entry {
            Ok(path) => {
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Some(name) = parse_pkg_name(&path) {
                        packages.push(PackageInfo {
                            name,
                            path,
                            mtime: metadata.modified()?,
                            size: metadata.len(),
                        });
                    }
                }
            }
            Err(e) => warn!("Failed to read: {:?}", e),
        }
    }
    
    Ok(packages)
}

fn group_by_name(packages: Vec<PackageInfo>) -> HashMap<String, Vec<PackageInfo>> {
    let mut groups: HashMap<String, Vec<PackageInfo>> = HashMap::new();
    
    for pkg in packages {
        groups.entry(pkg.name.clone()).or_insert_with(Vec::new).push(pkg);
    }
    
    groups
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    let cache_dir = args.cache_dir.unwrap_or_else(|| PathBuf::from("/var/cache/pacman/pkg"));
    
    if !cache_dir.exists() {
        return Err(anyhow::anyhow!("Cache directory does not exist: {}", cache_dir.display()));
    }

    info!("Scanning cache directory: {}", cache_dir.display());
    
    let packages = collect_packages(&cache_dir)?;
    let total_size: u64 = packages.iter().map(|p| p.size).sum();
    
    info!("Found {} packages ({} GB)", 
          packages.len(),
          total_size as f64 / (1024.0 * 1024.0 * 1024.0));

    let mut groups = group_by_name(packages);
    let mut to_delete: Vec<PackageInfo> = Vec::new();
    let mut freed_space: u64 = 0;

    for (_name, mut pkgs) in groups {
        // Sort by mtime descending (newest first)
        pkgs.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        
        // Keep only N newest
        let keep_count = args.keep as usize;
        
        if pkgs.len() <= keep_count {
            continue;
        }
        
        for pkg in pkgs.into_iter().skip(keep_count) {
            to_delete.push(pkg);
        }
    }

    if to_delete.is_empty() {
        info!("Nothing to clean");
        return Ok(());
    }

    let total_to_free: u64 = to_delete.iter().map(|p| p.size).sum();
    
    info!("Would free {:.2} MB by removing {} packages", 
          total_to_free as f64 / (1024.0 * 1024.0),
          to_delete.len());

    if args.dry_run {
        info!("DRY RUN - No files deleted");
        for pkg in &to_delete {
            println!("Would delete: {}", pkg.path.display());
        }
    } else {
        // Require confirmation for production use
        print!("Delete {} packages? [y/N]: ", to_delete.len());
        std::io::Write::flush(&mut std::io::stdout())?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            info!("Aborted");
            return Ok(());
        }

        for pkg in &to_delete {
            match fs::remove_file(&pkg.path) {
                Ok(_) => {
                    info!("Deleted: {}", pkg.path.display());
                    freed_space += pkg.size;
                }
                Err(e) => {
                    warn!("Failed to delete {:?}: {}", pkg.path, e);
                }
            }
        }
    }

    println!();
    println!("Cleanup complete!");
    println!("  Packages removed: {}", to_delete.len());
    println!("  Space freed: {:.2} MB", freed_space as f64 / (1024.0 * 1024.0));

    Ok(())
}
