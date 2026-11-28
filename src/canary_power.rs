//! Seeded canary-release power: a regressed agent lowers every task's
//! per-trial success by `d`; find the smallest drop on a grid where a paired
//! two-sided z-test on per-task pass^k estimates (C(c,k)/C(n,k)) reaches 0.80
//! power at alpha 0.05, for k of 3, 4 and 5 and several canary task counts.

use crate::mutate::Rng;
use serde_json::{json, Value};

const TRIALS: usize = 8;
const KS: [usize; 3] = [3, 4, 5];
const NS: [usize; 4] = [25, 50, 100, 200];
const DROPS: [f64; 10] = [0.02, 0.04, 0.06, 0.08, 0.10, 0.12, 0.14, 0.16, 0.18, 0.20];
const Z_CRIT: f64 = 1.959964;
const POWER_TARGET: f64 = 0.80;

fn unit(r: &mut Rng) -> f64 {
    (r.next() >> 11) as f64 / (1u64 << 53) as f64
}

fn choose(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// Unbiased pass^k estimate from `c` successes in `n` trials.
fn pass_k_est(c: usize, n: usize, k: usize) -> f64 {
    choose(c, k) / choose(n, k)
}

fn paired_z(diffs: &[f64]) -> f64 {
    let n = diffs.len() as f64;
    let m = diffs.iter().sum::<f64>() / n;
    let var = diffs.iter().map(|d| (d - m).powi(2)).sum::<f64>() / (n - 1.0);
    if var == 0.0 { 0.0 } else { m / (var / n).sqrt() }
}

fn successes(r: &mut Rng, p: f64) -> usize {
    (0..TRIALS).filter(|_| unit(r) < p).count()
}

fn power(r: &mut Rng, n: usize, k: usize, d: f64, reps: usize) -> f64 {
    let mut hits = 0usize;
    for _ in 0..reps {
        let diffs: Vec<f64> = (0..n)
            .map(|_| {
                let p = 0.6 + 0.35 * unit(r);
                let q = (p - d).max(0.0);
                let a = pass_k_est(successes(r, p), TRIALS, k);
                let b = pass_k_est(successes(r, q), TRIALS, k);
                a - b
            })
            .collect();
        if paired_z(&diffs).abs() > Z_CRIT {
            hits += 1;
        }
    }
    hits as f64 / reps as f64
}

pub fn run(seed: u64, replicates: usize) -> Value {
    let mut r = Rng(seed);
    let mut out = serde_json::Map::new();
    let mut curves = Vec::new();
    for &k in &KS {
        for &n in &NS {
            let powers: Vec<f64> = DROPS.iter().map(|&d| power(&mut r, n, k, d, replicates)).collect();
            let min = DROPS.iter().zip(&powers).find(|(_, &p)| p >= POWER_TARGET).map(|(&d, _)| json!(d)).unwrap_or(Value::Null);
            out.insert(format!("min_detectable_drop_k{k}_n{n}"), min);
            curves.push(json!({ "k": k, "tasks": n, "power": powers }));
        }
    }
    out.insert("drop_grid".into(), json!(DROPS));
    out.insert("curves".into(), json!(curves));
    out.insert("replicates".into(), json!(replicates));
    out.insert("trials".into(), json!(TRIALS));
    out.insert("alpha".into(), json!(0.05));
    out.insert("power_target".into(), json!(POWER_TARGET));
    json!({ "power": Value::Object(out) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run(7, 5), run(7, 5));
    }

    #[test]
    fn metric_keys_present_and_power_in_unit_interval() {
        let a = run(7, 5);
        assert!(a["power"].get("min_detectable_drop_k3_n100").is_some());
        assert!(a["power"].get("min_detectable_drop_k5_n100").is_some());
        for c in a["power"]["curves"].as_array().unwrap() {
            for p in c["power"].as_array().unwrap() {
                assert!((0.0..=1.0).contains(&p.as_f64().unwrap()));
            }
        }
    }

    #[test]
    fn estimator_edges() {
        assert_eq!(pass_k_est(8, 8, 5), 1.0);
        assert_eq!(pass_k_est(4, 8, 5), 0.0);
        assert!((pass_k_est(6, 8, 3) - 20.0 / 56.0).abs() < 1e-12);
    }

    #[test]
    fn paired_z_zero_variance() {
        assert_eq!(paired_z(&[0.0, 0.0, 0.0]), 0.0);
        assert!(paired_z(&[0.5, 0.2, 0.4]) > 0.0);
    }
}
