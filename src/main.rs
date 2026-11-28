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
    /// Seeded single-field mutation study; writes a results JSON.
    Mutate {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 200)]
        per_field: usize,
        #[arg(long, default_value_t = 200)]
        controls: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// Seeded floating-reference audit; writes a results JSON.
    PinAudit {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 200)]
        per_class: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// Seeded pass^k baseline for degenerate responders; writes a results JSON.
    PassKBaseline {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 200)]
        tasks: usize,
        #[arg(long, default_value_t = 8)]
        trials: usize,
        #[arg(long, default_value_t = 0.05)]
        floor: f64,
        #[arg(long)]
        out: PathBuf,
    },
    /// Seeded vacuous-task floor sweep for pass^k; writes a results JSON.
    FloorSweep {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 400)]
        tasks: usize,
        #[arg(long, default_value_t = 16)]
        trials: usize,
        #[arg(long, default_value_t = 0.05)]
        margin: f64,
        #[arg(long)]
        out: PathBuf,
    },
    /// Seeded scaffold-version spread study; writes a results JSON.
    ScaffoldSpread {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 200)]
        tasks: usize,
        #[arg(long, default_value_t = 8)]
        trials: usize,
        #[arg(long, default_value_t = 6)]
        models: usize,
        #[arg(long, default_value_t = 10)]
        versions: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// Seeded harness-version shift detection power study; writes a results JSON.
    ShiftDetect {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 400)]
        replicates: usize,
        #[arg(long)]
        out: PathBuf,
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
        Cmd::Mutate { seed, per_field, controls, out } => {
            let t = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let mut res = harnesspin::mutate::run(seed, per_field, controls);
            let (d, h, m, s) = (started / 86400, started % 86400 / 3600, started % 3600 / 60, started % 60);
            // civil-from-days (Hinnant)
            let z = d as i64 + 719468;
            let era = z.div_euclid(146097);
            let doe = z.rem_euclid(146097);
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let day = doy - (153 * mp + 2) / 5 + 1;
            let mon = if mp < 10 { mp + 3 } else { mp - 9 };
            let yr = yoe + era * 400 + (mon <= 2) as i64;
            res = serde_json::json!({
                "mutation": res,
                "provenance": {
                    "started_at": format!("{yr:04}-{mon:02}-{day:02}T{h:02}:{m:02}:{s:02}Z"),
                    "seeds": [seed],
                    "command": format!("harnesspin mutate --seed {seed} --per-field {per_field} --controls {controls} --out {}", out.display()),
                    "hardware": "cpu",
                    "wall_s": t.elapsed().as_secs_f64(),
                    "data_sources": [{"url": "synthetic: seeded generator in-repo", "license": "MIT"}],
                }
            });
            std::fs::write(&out, serde_json::to_string_pretty(&res)? + "\n")?;
        }
        Cmd::PinAudit { seed, per_class, out } => {
            let t = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let res = harnesspin::pin_audit::run(seed, per_class);
            let (d, h, m, s) = (started / 86400, started % 86400 / 3600, started % 3600 / 60, started % 60);
            let z = d as i64 + 719468;
            let era = z.div_euclid(146097);
            let doe = z.rem_euclid(146097);
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let day = doy - (153 * mp + 2) / 5 + 1;
            let mon = if mp < 10 { mp + 3 } else { mp - 9 };
            let yr = yoe + era * 400 + (mon <= 2) as i64;
            let res = serde_json::json!({
                "pin_audit": res,
                "provenance": {
                    "started_at": format!("{yr:04}-{mon:02}-{day:02}T{h:02}:{m:02}:{s:02}Z"),
                    "seeds": [seed],
                    "command": format!("harnesspin pin-audit --seed {seed} --per-class {per_class} --out {}", out.display()),
                    "hardware": "cpu",
                    "wall_s": t.elapsed().as_secs_f64(),
                    "data_sources": [{"url": "synthetic: seeded generator in-repo", "license": "MIT"}],
                }
            });
            std::fs::write(&out, serde_json::to_string_pretty(&res)? + "\n")?;
        }
        Cmd::PassKBaseline { seed, tasks, trials, floor, out } => {
            let t = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let res = harnesspin::pass_k::run(seed, tasks, trials, floor);
            let (d, h, m, s) = (started / 86400, started % 86400 / 3600, started % 3600 / 60, started % 60);
            let z = d as i64 + 719468;
            let era = z.div_euclid(146097);
            let doe = z.rem_euclid(146097);
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let day = doy - (153 * mp + 2) / 5 + 1;
            let mon = if mp < 10 { mp + 3 } else { mp - 9 };
            let yr = yoe + era * 400 + (mon <= 2) as i64;
            let res = serde_json::json!({
                "pass_k_baseline": res,
                "provenance": {
                    "started_at": format!("{yr:04}-{mon:02}-{day:02}T{h:02}:{m:02}:{s:02}Z"),
                    "seeds": [seed],
                    "command": format!("harnesspin pass-k-baseline --seed {seed} --tasks {tasks} --trials {trials} --floor {floor} --out {}", out.display()),
                    "hardware": "cpu",
                    "wall_s": t.elapsed().as_secs_f64(),
                    "data_sources": [{"url": "synthetic: seeded generator in-repo", "license": "MIT"}],
                }
            });
            std::fs::write(&out, serde_json::to_string_pretty(&res)? + "\n")?;
        }
        Cmd::FloorSweep { seed, tasks, trials, margin, out } => {
            let t = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let res = harnesspin::floor_sweep::run(seed, tasks, trials, margin);
            let (d, h, m, s) = (started / 86400, started % 86400 / 3600, started % 3600 / 60, started % 60);
            let z = d as i64 + 719468;
            let era = z.div_euclid(146097);
            let doe = z.rem_euclid(146097);
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let day = doy - (153 * mp + 2) / 5 + 1;
            let mon = if mp < 10 { mp + 3 } else { mp - 9 };
            let yr = yoe + era * 400 + (mon <= 2) as i64;
            let res = serde_json::json!({
                "floor_sweep": res["floor_sweep"],
                "tasks": res["tasks"], "trials": res["trials"], "margin": res["margin"],
                "provenance": {
                    "started_at": format!("{yr:04}-{mon:02}-{day:02}T{h:02}:{m:02}:{s:02}Z"),
                    "seeds": [seed],
                    "command": format!("harnesspin floor-sweep --seed {seed} --tasks {tasks} --trials {trials} --margin {margin} --out {}", out.display()),
                    "hardware": "cpu",
                    "wall_s": t.elapsed().as_secs_f64(),
                    "data_sources": [{"url": "synthetic: seeded generator in-repo", "license": "MIT"}],
                }
            });
            std::fs::write(&out, serde_json::to_string_pretty(&res)? + "\n")?;
        }
        Cmd::ScaffoldSpread { seed, tasks, trials, models, versions, out } => {
            let t = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let mut res = harnesspin::scaffold_spread::run(seed, tasks, trials, models, versions);
            let (d, h, m, s) = (started / 86400, started % 86400 / 3600, started % 3600 / 60, started % 60);
            let z = d as i64 + 719468;
            let era = z.div_euclid(146097);
            let doe = z.rem_euclid(146097);
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let day = doy - (153 * mp + 2) / 5 + 1;
            let mon = if mp < 10 { mp + 3 } else { mp - 9 };
            let yr = yoe + era * 400 + (mon <= 2) as i64;
            res["provenance"] = serde_json::json!({
                "started_at": format!("{yr:04}-{mon:02}-{day:02}T{h:02}:{m:02}:{s:02}Z"),
                "seeds": [seed],
                "command": format!("harnesspin scaffold-spread --seed {seed} --tasks {tasks} --trials {trials} --models {models} --versions {versions} --out {}", out.display()),
                "hardware": "cpu",
                "wall_s": t.elapsed().as_secs_f64(),
                "data_sources": [{"url": "synthetic: seeded generator in-repo", "license": "MIT"}],
            });
            std::fs::write(&out, serde_json::to_string_pretty(&res)? + "\n")?;
        }
        Cmd::ShiftDetect { seed, replicates, out } => {
            let t = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let mut res = harnesspin::shift_detect::run(seed, replicates);
            let (d, h, m, s) = (started / 86400, started % 86400 / 3600, started % 3600 / 60, started % 60);
            let z = d as i64 + 719468;
            let era = z.div_euclid(146097);
            let doe = z.rem_euclid(146097);
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let day = doy - (153 * mp + 2) / 5 + 1;
            let mon = if mp < 10 { mp + 3 } else { mp - 9 };
            let yr = yoe + era * 400 + (mon <= 2) as i64;
            res["provenance"] = serde_json::json!({
                "started_at": format!("{yr:04}-{mon:02}-{day:02}T{h:02}:{m:02}:{s:02}Z"),
                "seeds": [seed],
                "command": format!("harnesspin shift-detect --seed {seed} --replicates {replicates} --out {}", out.display()),
                "hardware": "cpu",
                "wall_s": t.elapsed().as_secs_f64(),
                "data_sources": [{"url": "synthetic: seeded generator in-repo", "license": "MIT"}],
            });
            std::fs::write(&out, serde_json::to_string_pretty(&res)? + "\n")?;
        }
        Cmd::Version => println!("harnesspin {}", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}
