//! harnesspin CLI.
//!
//! Four verbs: `canon` to print the canonical SHA-256 of a manifest,
//! `compare` to list the divergences between a declared and an
//! observed manifest, `verify` to apply the pre-reg gate and exit on
//! verdict, and `version` for the banner.

use clap::{Parser, Subcommand};
use harnesspin::{
    canonical_sha256_hex, compare, is_degenerate, verify, RunManifest, Verdict,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "harnesspin", version, about = "Pre-registered eval-harness run manifest")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Canonical SHA-256 of a manifest. Prints the lowercase hex.
    /// Exit 2 if the manifest is degenerate (pre-reg §7).
    Canon {
        manifest: PathBuf,
    },
    /// List the divergences between a declared and an observed
    /// manifest. Prints a JSON array; exit 0 if matched, 1 if
    /// anything differs, 2 if either side is degenerate.
    Compare {
        declared: PathBuf,
        observed: PathBuf,
    },
    /// Apply the pre-reg gate. Prints the Verdict as JSON. Exit 0 on
    /// Matched, 1 on DivergedPinned, 2 on Unverified. CI gates read
    /// the exit code.
    Verify {
        declared: PathBuf,
        observed: PathBuf,
    },
    /// Print the version banner and exit.
    Version,
}

fn read_manifest(p: &PathBuf) -> anyhow::Result<RunManifest> {
    Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Canon { manifest } => {
            let m = read_manifest(&manifest)?;
            if let Some(r) = is_degenerate(&m) {
                eprintln!("manifest is degenerate: {r}");
                std::process::exit(2);
            }
            println!("{}", canonical_sha256_hex(&m));
        }
        Cmd::Compare { declared, observed } => {
            let a = read_manifest(&declared)?;
            let b = read_manifest(&observed)?;
            if let Some(r) = is_degenerate(&a) {
                eprintln!("declared manifest is degenerate: {r}");
                std::process::exit(2);
            }
            if let Some(r) = is_degenerate(&b) {
                eprintln!("observed manifest is degenerate: {r}");
                std::process::exit(2);
            }
            let divs = compare(&a, &b);
            println!("{}", serde_json::to_string_pretty(&divs)?);
            if !divs.is_empty() {
                std::process::exit(1);
            }
        }
        Cmd::Verify { declared, observed } => {
            let a = read_manifest(&declared)?;
            let b = read_manifest(&observed)?;
            let v = verify(&a, &b);
            println!("{}", serde_json::to_string_pretty(&v)?);
            match v {
                Verdict::Matched => {}
                Verdict::DivergedPinned { .. } => std::process::exit(1),
                Verdict::Unverified { .. } => std::process::exit(2),
            }
        }
        Cmd::Version => println!("harnesspin {}", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}
