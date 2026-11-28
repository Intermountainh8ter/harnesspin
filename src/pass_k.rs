//! Seeded pass^k baseline: the estimator C(c,k)/C(n,k) averaged over
//! synthetic tasks, for responders that carry no model signal.

use crate::mutate::Rng;
use serde_json::{json, Value};

/// C(c,k)/C(n,k) as a product, no overflow: prod_{i<k} (c-i)/(n-i).
pub fn pass_hat_k(c: usize, n: usize, k: usize) -> f64 {
    if k > n || k > c {
        return 0.0;
    }
    (0..k).map(|i| (c - i) as f64 / (n - i) as f64).product()
}

fn curve(rate: f64, tasks: usize, trials: usize, r: &mut Rng) -> Vec<f64> {
    let counts: Vec<usize> = (0..tasks)
        .map(|_| {
            (0..trials)
                .filter(|_| (r.next() >> 11) as f64 / (1u64 << 53) as f64 <= rate && rate > 0.0)
                .count()
        })
        .collect();
    (1..=trials)
        .map(|k| counts.iter().map(|&c| pass_hat_k(c, trials, k)).sum::<f64>() / tasks as f64)
        .collect()
}

fn first_below(c: &[f64], floor: f64) -> Value {
    match c.iter().position(|&v| v < floor) {
        Some(i) => json!(i + 1),
        None => Value::Null,
    }
}

pub fn run(seed: u64, tasks: usize, trials: usize, floor: f64) -> Value {
    let mut r = Rng(seed);
    let mut out = serde_json::Map::new();
    for (name, rate) in [("always_fail", 0.0), ("coinflip", 0.5), ("fixed_0_9", 0.9), ("fixed_0_7", 0.7)] {
        let c = curve(rate, tasks, trials, &mut r);
        let mono = c.windows(2).all(|w| w[1] <= w[0]);
        out.insert(
            name.into(),
            json!({
                "rate": rate,
                "pass_hat_k": c,
                "k_below_floor": first_below(&c, floor),
                "monotone_non_increasing": mono,
            }),
        );
    }
    json!({ "tasks": tasks, "trials": trials, "floor": floor, "responders": out.keys().collect::<Vec<_>>() , "__merge": null })
        .as_object()
        .map(|meta| {
            let mut o = out;
            for (k, v) in meta {
                if k != "__merge" {
                    o.insert(k.clone(), v.clone());
                }
            }
            Value::Object(o)
        })
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_matches_binomials() {
        assert_eq!(pass_hat_k(8, 8, 8), 1.0);
        assert_eq!(pass_hat_k(0, 8, 1), 0.0);
        assert!((pass_hat_k(4, 8, 2) - 6.0 / 28.0).abs() < 1e-12);
        assert_eq!(pass_hat_k(3, 8, 4), 0.0);
    }

    #[test]
    fn deterministic_and_always_fail_floor_at_one() {
        let a = run(7, 50, 8, 0.05);
        assert_eq!(a, run(7, 50, 8, 0.05));
        assert_eq!(a["always_fail"]["k_below_floor"], 1);
        assert!(a["fixed_0_9"]["pass_hat_k"][0].as_f64().unwrap() > 0.5);
    }
}
