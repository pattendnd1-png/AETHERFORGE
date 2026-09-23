//! AetherForge rankmirrors replacement
//! Speed-optimize mirrorlist

use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Rank mirrors by speed
#[derive(Parser, Debug)]
#[command(name = "rankmirrors-rs")]
#[command(author = "AetherForge Team")]
#[command(version = "1.0.0")]
#[command(about = "Speed-test and rank pacman mirrors")]
struct Args {
    /// Maximum number of mirrors to keep
    #[arg(short, long, default_value_t = 5)]
    max_mirrors: u32,

    /// Mirror list file
    #[arg(short, long, default_value = "/etc/pacman.d/mirrorlist")]
    input: PathBuf,

    /// Output file (default: overwrite input)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Timeout per mirror (seconds)
    #[arg(long, default_value_t = 10)]
    timeout_secs: u64,

    /// Country code filter (e.g., US)
    #[arg(short, long)]
    country: Option<String>,
}

#[derive(Debug, Clone)]
struct Mirror {
    url: String,
    country: String,
    response_time_ms: Option<u64>,
}

fn test_mirror_speed(url: &str, timeout: Duration) -> Option<u64> {
    let start = Instant::now();
    
    // Parse host from URL for TCP connection
    let host = url.trim_end_matches('/');
    let host_part = host.trim_start_matches("http://").trim_start_matches("https://");
    let host_port = host_part.split('/').next().unwrap_or(host_part);
    
    let addr = host_port.parse::<std::net::SocketAddr>().ok()?;
    match std::net::TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => Some(start.elapsed().as_millis() as u64),
        Err(_) => None,
    }
}

fn parse_mirror_list(input: &PathBuf) -> Result<Vec<Mirror>> {
    let content = fs::read_to_string(input)?;
    let mut mirrors = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        
        if line.starts_with("#Server") || line.starts_with("Server ") {
            let url = line.strip_prefix("#").unwrap_or(line).split('=').last().unwrap_or(line).trim();
            if !url.is_empty() && !url.starts_with('#') {
                // Extract country from URL (basic parsing)
                let parts: Vec<&str> = url.split('/').collect();
                let country = parts.get(parts.len() - 4).unwrap_or(&"unknown").to_string();
                
                mirrors.push(Mirror {
                    url: url.to_string(),
                    country,
                    response_time_ms: None,
                });
            }
        }
    }

    Ok(mirrors)
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.input.exists() {
        return Err(anyhow::anyhow!("Mirror list not found: {}", args.input.display()));
    }

    println!("Reading mirrors from: {}", args.input.display());
    
    let mut mirrors = parse_mirror_list(&args.input)?;
    println!("Found {} mirror(s)", mirrors.len());

    if let Some(country) = &args.country {
        mirrors.retain(|m| m.country.eq_ignore_ascii_case(country));
        println!("Filtered to {}: {} mirror(s)", country, mirrors.len());
    }

    if mirrors.is_empty() {
        println!("No mirrors found after filtering.");
        return Ok(());
    }

    println!("\nTesting mirror speeds (this may take a minute)...");
    
    let timeout = Duration::from_secs(args.timeout_secs);
    for mirror in &mut mirrors {
        print!("Testing: {} ... ", mirror.country);
        let result = test_mirror_speed(&mirror.url, timeout);
        mirror.response_time_ms = result;
        match result {
            Some(ms) => println!("{}ms", ms),
            None => println!("timeout"),
        }
    }

    // Sort by response time (fastest first), unresponsive at end
    mirrors.sort_by(|a, b| {
        let a_time = a.response_time_ms.unwrap_or(u64::MAX);
        let b_time = b.response_time_ms.unwrap_or(u64::MAX);
        a_time.cmp(&b_time)
    });

    // Take top N
    let top_mirrors: Vec<_> = mirrors.into_iter().take(args.max_mirrors as usize).collect();

    println!("\nTop {} mirrors by speed:", top_mirrors.len());
    for (i, mirror) in top_mirrors.iter().enumerate() {
        let time = mirror.response_time_ms.map_or("timeout".to_string(), |ms| format!("{}ms", ms));
        println!("  {}. {} ({})", i + 1, time, mirror.country);
    }

    let output = args.output.unwrap_or_else(|| args.input.clone());
    let mut content = String::new();

    for mirror in &top_mirrors {
        content.push_str(&format!("Server = {}\n", mirror.url));
    }

    fs::write(&output, content)?;
    println!("\n✓ Updated: {}", output.display());
    println!("\nTip: Test the new mirrorlist with:");
    println!("  sudo pacman -Sy");

    Ok(())
}
