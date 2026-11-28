//! Seeded floor sweep: a share of vacuously satisfiable tasks sets a
//! pass^k floor that a do-nothing responder reaches at every k; report
//! the k at which an agent's headline pass^k comes within a margin of it.

use crate::mutate::Rng;
use crate::pass_k::pass_hat_k;
use serde_json::{json, Value};

fn unit(r: &mut Rng) -> f64 {
    (r.next() >> 11) as f64 / (1u64 << 53) as f64
}

fn arm(share: f64, seed: u64, tasks: usize, trials: usize, margin: f64) -> Value {
    let mut r = Rng(seed);
    let n_vac = (share * tasks as f64).round() as usize;
    // First n_vac tasks are vacuous: both responders pass every trial.
    // The agent's per-trial success on the rest is U(0.2, 1.0).
    let agent: Vec<usize> = (0..tasks)
        .map(|t| {
            if t < n_vac {
                trials
            } else {
                let p = 0.2 + 0.8 * unit(&mut r);
                (0..trials).filter(|_| unit(&mut r) <= p).count()
            }
        })
        .collect();
    let nothing: Vec<usize> = (0..tasks).map(|t| if t < n_vac { trials } else { 0 }).collect();
    let mean = |cs: &[usize], k: usize| cs.iter().map(|&c| pass_hat_k(c, trials, k)).sum::<f64>() / tasks as f64;
    let agent_curve: Vec<f64> = (1..=trials).map(|k| mean(&agent, k)).collect();
    let floor_curve: Vec<f64> = (1..=trials).map(|k| mean(&nothing, k)).collect();
    let k_at_floor = agent_curve
        .iter()
        .zip(&floor_curve)
        .position(|(a, f)| a - f <= margin)
        .map(|i| json!(i + 1))
        .unwrap_or(Value::Null);
    let vac = n_vac as f64 / tasks as f64;
    json!({
        "vacuous_share": share,
        "vacuous_tasks": n_vac,
        "do_nothing_pass_hat_k": floor_curve,
        "do_nothing_equals_share": floor_curve.iter().all(|&f| (f - vac).abs() < 1e-12),
        "agent_pass_hat_k": agent_curve,
        "margin": margin,
        "k_at_floor": k_at_floor,
    })
}

pub fn run(seed: u64, tasks: usize, trials: usize, margin: f64) -> Value {
    let arms: Vec<Value> = [0.38, 0.06].iter().map(|&s| arm(s, seed, tasks, trials, margin)).collect();
    json!({ "tasks": tasks, "trials": trials, "margin": margin, "floor_sweep": arms })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_floor_equals_share() {
        let a = run(7, 100, 8, 0.05);
        assert_eq!(a, run(7, 100, 8, 0.05));
        for i in 0..2 {
            assert_eq!(a["floor_sweep"][i]["do_nothing_equals_share"], true);
        }
        assert_eq!(a["floor_sweep"][0]["vacuous_tasks"], 38);
    }

    #[test]
    fn agent_curve_non_increasing_and_above_floor() {
        let a = run(7, 100, 8, 0.05);
        let c = a["floor_sweep"][0]["agent_pass_hat_k"].as_array().unwrap();
        assert!(c.windows(2).all(|w| w[1].as_f64().unwrap() <= w[0].as_f64().unwrap()));
        assert!(c[7].as_f64().unwrap() >= 0.38 - 1e-12);
    }
}
