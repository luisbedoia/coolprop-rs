//! Round trips: a state defined one way (p–T in one phase, p–x on and inside
//! the dome) and solved back from every input pair must be the same state,
//! or an error. Never another state: that is a wrong answer given as a right
//! one. Errors are fine, CoolProp cannot solve every pair everywhere (p–T
//! inside the dome does not fix a state at all).

use std::collections::BTreeMap;

use coolprop::schema;
use coolprop::{Fluid, Input, InputKind, State, Variant};

fn value_of(s: &State, kind: InputKind) -> Option<f64> {
    Some(match kind {
        InputKind::Pressure => s.pressure(),
        InputKind::Temperature => s.temperature(),
        InputKind::Density => s.density(),
        InputKind::Enthalpy => s.enthalpy(),
        InputKind::Entropy => s.entropy(),
        InputKind::InternalEnergy => s.internal_energy(),
        InputKind::Quality => s.quality()?,
    })
}

fn lin(a: f64, b: f64, i: usize, n: usize) -> f64 {
    a + (b - a) * i as f64 / (n - 1) as f64
}

fn log(a: f64, b: f64, i: usize, n: usize) -> f64 {
    a * (b / a).powf(i as f64 / (n - 1) as f64)
}

/// Reference states over the whole diagram: a p–T grid in single phase
/// (kept 1 K off saturation) and p–x states on and inside the dome.
fn reference_states(f: &Fluid) -> Vec<(String, State)> {
    let d = f.data();
    let c = f.critical();
    let (t_lo, t_hi) = (d.t_triple * 1.02, (2.0 * c.temperature).min(0.95 * d.t_max));
    let (p_lo, p_hi) = (
        (2.0 * d.p_triple).max(500.0),
        (3.0 * c.pressure).min(0.5 * d.p_max),
    );
    let mut out = Vec::new();
    for i in 0..8 {
        for j in 0..8 {
            let (t, p) = (lin(t_lo, t_hi, i, 8), log(p_lo, p_hi, j, 8));
            let Ok(s) = f.state(Input::Pressure(p), Input::Temperature(t)) else {
                continue;
            };
            if p < c.pressure {
                let near = [0.0, 1.0].iter().any(|&x| {
                    f.state(Input::Pressure(p), Input::Quality(x))
                        .is_ok_and(|sat| (sat.temperature() - t).abs() < 1.0)
                });
                if near {
                    continue;
                }
            }
            out.push((format!("pT {:.0}K {:.3e}Pa", t, p), s));
        }
    }
    for j in 0..6 {
        let p = log(p_lo.max(d.p_triple * 2.0), 0.95 * c.pressure, j, 6);
        for x in [0.0, 0.1, 0.5, 0.9, 1.0] {
            if let Ok(s) = f.state(Input::Pressure(p), Input::Quality(x)) {
                out.push((format!("px {:.3e}Pa x={x}", p), s));
            }
        }
    }
    out
}

#[derive(Default)]
struct Tally {
    ok: usize,
    failed: usize,
    wrong: usize,
    worst: f64,
    example: String,
}

/// Largest relative error (T, h, s) still taken as the same state: the
/// solvers' own convergence, e.g. at the triple point.
const SAME_STATE: f64 = 1e-4;

#[test]
fn every_pair_gives_back_the_state_or_an_error() {
    let mut wrong = 0;
    let mut report = String::new();
    for &variant in Variant::ALL {
        let f = Fluid::new(variant).unwrap();
        let refs = reference_states(&f);
        let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
        for (label, s) in &refs {
            let region = if s.quality().is_some() {
                "dome"
            } else {
                "1-phase"
            };
            for (a, b) in schema::pairs() {
                let (Some(va), Some(vb)) = (value_of(s, a), value_of(s, b)) else {
                    continue;
                };
                let fresh = Fluid::new(variant).unwrap();
                let t = tallies
                    .entry(format!("{:>15} {:>15} {region}", a.name(), b.name()))
                    .or_default();
                match fresh.state(a.with_value(va), b.with_value(vb)) {
                    Err(e) => {
                        t.failed += 1;
                        if t.example.is_empty() {
                            t.example = format!(
                                "FAIL at {label}: {}",
                                e.to_string().chars().take(70).collect::<String>()
                            );
                        }
                    }
                    Ok(g) => {
                        // T, h and s identify the state, and every pair leaves
                        // at least one of them free to tell a wrong one. Not
                        // p (a liquid's moves 1e-3 from (h, s) at the triple
                        // point) nor ρ (on the liquid line at low pressure, x
                        // off by 1e-9 moves it by 1e-4). h and s are near 0
                        // at some reference states: their errors are relative
                        // to at least 10 kJ/kg and 10 J/(kg·K).
                        let rel = |x: f64, y: f64, floor: f64| (x - y).abs() / y.abs().max(floor);
                        let err = [
                            rel(g.temperature(), s.temperature(), 1.0),
                            rel(g.enthalpy(), s.enthalpy(), 1e4),
                            rel(g.entropy(), s.entropy(), 10.0),
                        ]
                        .into_iter()
                        .fold(0.0, f64::max);
                        if err < SAME_STATE {
                            t.ok += 1;
                        } else {
                            t.wrong += 1;
                            if err > t.worst {
                                t.worst = err;
                                t.example = format!(
                                    "WRONG at {label}: rel {err:.1e}; got phase {:?} x={:?} T={:.2} p={:.0} (ref T={:.2} p={:.0})",
                                    g.phase(),
                                    g.quality(),
                                    g.temperature(),
                                    g.pressure(),
                                    s.temperature(),
                                    s.pressure()
                                );
                            }
                        }
                    }
                }
            }
        }
        report += &format!("== {} ({} reference states)\n", variant.name(), refs.len());
        for (k, t) in tallies.iter().filter(|(_, t)| t.failed + t.wrong > 0) {
            wrong += t.wrong;
            report += &format!(
                "   {k}: ok {} failed {} WRONG {}  {}\n",
                t.ok, t.failed, t.wrong, t.example
            );
        }
    }
    assert!(wrong == 0, "{wrong} solves gave another state:\n{report}");
}
