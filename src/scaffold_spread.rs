//! Seeded scaffold-spread study: each scaffold version perturbs per-task
//! success probabilities; compare the ranking-flip fraction between models
//! scored under a randomly drawn version (unpinned) and one pinned version.

use crate::mutate::Rng;
use serde_json::{json, Value};

const REPS: usize = 200;
const SKILL_STEP: f64 = 0.03;
const GLOBAL_OFFSET: f64 = 0.05;
const TASK_PERTURB: f64 = 0.15;

fn unit(r: &mut Rng) -> f64 {
    (r.next() >> 11) as f64 / (1u64 << 53) as f64
}

fn flip_fraction(scores: &[f64]) -> f64 {
    // Models are indexed by increasing true skill; a flip is a lower-skill
    // model scoring above a higher-skill one (ties count half).
    let mut flips = 0.0;
    let mut pairs = 0.0;
    for i in 0..scores.len() {
        for j in i + 1..scores.len() {
            pairs += 1.0;
            if scores[i] > scores[j] {
                flips += 1.0;
            } else if scores[i] == scores[j] {
                flips += 0.5;
            }
        }
    }
    if pairs == 0.0 { 0.0 } else { flips / pairs }
}

fn std(xs: &[f64]) -> f64 {
    let m = xs.iter().sum::<f64>() / xs.len() as f64;
    (xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64).sqrt()
}

pub fn run(seed: u64, tasks: usize, trials: usize, models: usize, versions: usize) -> Value {
    let mut r = Rng(seed);
    let (mut fu, mut fp) = (0.0, 0.0);
    let mut su = 0.0;
    for _ in 0..REPS {
        let base: Vec<f64> = (0..tasks).map(|_| 0.2 + 0.7 * unit(&mut r)).collect();
        let off: Vec<f64> = (0..versions).map(|_| GLOBAL_OFFSET * (2.0 * unit(&mut r) - 1.0)).collect();
        let delta: Vec<Vec<f64>> = (0..versions)
            .map(|_| (0..tasks).map(|_| TASK_PERTURB * (2.0 * unit(&mut r) - 1.0)).collect())
            .collect();
        let score = |m: usize, v: usize, r: &mut Rng| -> f64 {
            let mut s = 0usize;
            for t in 0..tasks {
                let p = (base[t] + SKILL_STEP * m as f64 + off[v] + delta[v][t]).clamp(0.0, 1.0);
                s += (0..trials).filter(|_| unit(r) < p).count();
            }
            s as f64 / (tasks * trials) as f64
        };
        let unpinned: Vec<f64> = (0..models)
            .map(|m| {
                let v = (r.next() % versions as u64) as usize;
                score(m, v, &mut r)
            })
            .collect();
        let pinned: Vec<f64> = (0..models).map(|m| score(m, 0, &mut r)).collect();
        fu += flip_fraction(&unpinned);
        fp += flip_fraction(&pinned);
        // Score spread of the middle model across all versions.
        let mid = models / 2;
        let across: Vec<f64> = (0..versions).map(|v| score(mid, v, &mut r)).collect();
        su += std(&across);
    }
    let n = REPS as f64;
    json!({
        "scaffold_spread": {
            "tasks": tasks, "trials": trials, "models": models, "versions": versions,
            "replicates": REPS,
            "skill_step": SKILL_STEP, "global_offset": GLOBAL_OFFSET, "task_perturbation": TASK_PERTURB,
            "ranking_flip_fraction_unpinned": fu / n,
            "ranking_flip_fraction_pinned": fp / n,
            "mean_score_std_across_versions": su / n,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run(7, 50, 4, 4, 5), run(7, 50, 4, 4, 5));
    }

    #[test]
    fn flip_fractions_in_unit_interval_and_flip_fraction_counts() {
        let a = run(7, 50, 4, 4, 5);
        for k in ["ranking_flip_fraction_unpinned", "ranking_flip_fraction_pinned"] {
            let v = a["scaffold_spread"][k].as_f64().unwrap();
            assert!((0.0..=1.0).contains(&v));
        }
        assert_eq!(flip_fraction(&[0.1, 0.2, 0.3]), 0.0);
        assert_eq!(flip_fraction(&[0.3, 0.2, 0.1]), 1.0);
        assert_eq!(flip_fraction(&[0.2, 0.2]), 0.5);
    }
}
