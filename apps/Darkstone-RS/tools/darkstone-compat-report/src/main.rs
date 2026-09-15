use std::{path::PathBuf, process::ExitCode};

use clap::Parser;
use darkstone_compat_report::{ScanError, scan_install, write_json_report};

#[derive(Debug, Parser)]
#[command(name = "darkstone-compat-report")]
#[command(about = "Read-only Darkstone installation compatibility scanner")]
struct Cli {
    #[arg(long)]
    install: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let report = match scan_install(&cli.install) {
        Ok(report) => report,
        Err(ScanError::RequiredArchiveParse { .. }) => {
            eprintln!("required DATA.MTF could not be parsed safely");
            return ExitCode::from(3);
        }
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };

    if let Some(path) = cli.output {
        if let Err(err) = write_json_report(&report, &path) {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    } else {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("failed to serialize report: {err}");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}
