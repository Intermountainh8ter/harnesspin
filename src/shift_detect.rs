//! Seeded harness-version shift detection: a harness change moves per-task
//! success probabilities by a known global shift plus a per-task perturbation;
//! find the smallest fixture on a grid where a paired one-trial z-test flags
//! the shift at 0.80 power and a 0.05 two-sided level.

use crate::mutate::Rng;
use serde_json::{json, Value};

const GRID: [usize; 8] = [50, 100, 200, 400, 800, 1600, 3200, 6400];
const SHIFTS: [f64; 2] = [0.05, 0.10];
const TASK_PERTURB: f64 = 0.15;
const Z_CRIT: f64 = 1.959964;
const POWER_TARGET: f64 = 0.80;

fn unit(r: &mut Rng) -> f64 {
    (r.next() >> 11) as f64 / (1u64 << 53) as f64
}

/// Paired z statistic on per-task success differences (b - a).
fn paired_z(diffs: &[i32]) -> f64 {
    let n = diffs.len() as f64;
    let m = diffs.iter().map(|&d| d as f64).sum::<f64>() / n;
    let var = diffs.iter().map(|&d| (d as f64 - m).powi(2)).sum::<f64>() / (n - 1.0);
    if var == 0.0 { 0.0 } else { m / (var / n).sqrt() }
}

fn power(r: &mut Rng, n: usize, shift: f64, reps: usize) -> f64 {
    let mut hits = 0usize;
    for _ in 0..reps {
        let diffs: Vec<i32> = (0..n)
            .map(|_| {
                let base = 0.2 + 0.7 * unit(r);
                let pb = (base + shift + TASK_PERTURB * (2.0 * unit(r) - 1.0)).clamp(0.0, 1.0);
                let a = (unit(r) < base) as i32;
                let b = (unit(r) < pb) as i32;
                b - a
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
    for &shift in &SHIFTS {
        let powers: Vec<f64> = GRID.iter().map(|&n| power(&mut r, n, shift, replicates)).collect();
        let min = GRID.iter().zip(&powers).find(|(_, &p)| p >= POWER_TARGET).map(|(&n, _)| json!(n)).unwrap_or(Value::Null);
        let key = format!("min_tasks_shift_{}", format!("{shift:.2}").replace('.', "_"));
        out.insert(key, min);
        curves.push(json!({ "shift": shift, "power": powers }));
    }
    out.insert("grid".into(), json!(GRID));
    out.insert("curves".into(), json!(curves));
    out.insert("replicates".into(), json!(replicates));
    out.insert("alpha".into(), json!(0.05));
    out.insert("power_target".into(), json!(POWER_TARGET));
    out.insert("task_perturbation".into(), json!(TASK_PERTURB));
    json!({ "detection": Value::Object(out) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run(7, 20), run(7, 20));
    }

    #[test]
    fn metric_key_present_and_power_in_unit_interval() {
        let a = run(7, 20);
        assert!(a["detection"].get("min_tasks_shift_0_05").is_some());
        assert!(a["detection"].get("min_tasks_shift_0_10").is_some());
        for c in a["detection"]["curves"].as_array().unwrap() {
            for p in c["power"].as_array().unwrap() {
                assert!((0.0..=1.0).contains(&p.as_f64().unwrap()));
            }
        }
    }

    #[test]
    fn paired_z_sign_and_zero_variance() {
        assert!(paired_z(&[1, 1, 0, 1, 0, 1]) > 0.0);
        assert!(paired_z(&[-1, -1, 0, -1]) < 0.0);
        assert_eq!(paired_z(&[0, 0, 0]), 0.0);
    }
}
