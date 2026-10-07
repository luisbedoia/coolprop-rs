//! A `Fluid` session has no memory: solving a state gives the same result
//! whatever was solved before with it. CoolProp's `AbstractState` keeps the
//! phase of its last flash, which once made every pressure–temperature
//! flash fail after a density–quality one.

use coolprop::schema;
use coolprop::{Fluid, Input, InputKind, PropsError, State, Variant};

/// The value of an input in a solved state, if it has one.
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

/// States across the diagram: liquid, gas, two-phase, near saturation and
/// supercritical.
fn reference_states(f: &Fluid) -> Vec<State> {
    let c = f.critical();
    let (pc, tc) = (c.pressure, c.temperature);
    let t_sat = |p: f64| {
        f.state(Input::Pressure(p), Input::Quality(1.0))
            .unwrap()
            .temperature()
    };
    // Low pressure (about 1 atm for water) is where a stale phase broke
    // pressure–temperature flashes, for liquid and vapor alike.
    let (p_mid, p_low) = (0.1 * pc, (0.005 * pc).max(2.0 * f.data().p_triple));
    [
        (Input::Pressure(0.5 * pc), Input::Temperature(0.8 * tc)),
        (Input::Pressure(0.05 * pc), Input::Temperature(1.2 * tc)),
        (Input::Pressure(0.3 * pc), Input::Quality(0.4)),
        (
            Input::Pressure(p_mid),
            Input::Temperature(t_sat(p_mid) + 0.1),
        ),
        (
            Input::Pressure(p_low),
            Input::Temperature(t_sat(p_low) + 0.1),
        ),
        (
            Input::Pressure(p_low),
            Input::Temperature(t_sat(p_low) - 10.0),
        ),
        (Input::Pressure(p_low), Input::Quality(0.1)),
        (Input::Pressure(2.0 * pc), Input::Temperature(1.5 * tc)),
    ]
    .into_iter()
    .map(|(a, b)| f.state(a, b).unwrap())
    .collect()
}

type Outcome = Result<(f64, f64), String>;

fn outcome(r: Result<State, PropsError>) -> Outcome {
    r.map(|s| (s.density(), s.enthalpy()))
        .map_err(|e| e.to_string())
}

#[test]
fn a_session_solves_like_a_fresh_one_whatever_came_before() {
    let names = ["Water", "R134a", "CarbonDioxide", "R410A"];
    for variant in names.iter().filter_map(|n| Variant::from_name(n)) {
        let fluid = Fluid::new(variant).unwrap();
        // Every (reference state, input pair) as inputs, with the outcome
        // of a fresh session as the expectation.
        let mut cases: Vec<(Input, Input, Outcome)> = Vec::new();
        for s in reference_states(&fluid) {
            for (a, b) in schema::pairs() {
                let (Some(va), Some(vb)) = (value_of(&s, a), value_of(&s, b)) else {
                    continue;
                };
                let (ia, ib) = (a.with_value(va), b.with_value(vb));
                let fresh = Fluid::new(variant).unwrap();
                cases.push((ia, ib, outcome(fresh.state(ia, ib))));
            }
        }
        assert!(cases.len() > 60, "{} cases", cases.len());

        // Every ordered couple (A, B): B right after A in one session must
        // give what a fresh session gives. A flash in between can clear a
        // stale phase, so each B is checked directly after each A.
        let shared = Fluid::new(variant).unwrap();
        for (a1, a2, _) in &cases {
            for (b1, b2, expected) in &cases {
                let _ = shared.state(*a1, *a2);
                let got = outcome(shared.state(*b1, *b2));
                match (&got, expected) {
                    (Ok((d, h)), Ok((de, he))) => {
                        let rel = |x: f64, y: f64| (x - y).abs() / y.abs().max(1.0);
                        assert!(
                            rel(*d, *de) < 1e-6 && rel(*h, *he) < 1e-6,
                            "{}: {b1:?}, {b2:?} after {a1:?}, {a2:?} gave ρ={d}, h={h}; \
                             fresh ρ={de}, h={he}",
                            variant.name()
                        );
                    }
                    (Err(_), Err(_)) => {}
                    _ => panic!(
                        "{}: {b1:?}, {b2:?} after {a1:?}, {a2:?} gave {got:?}; \
                         a fresh session gives {expected:?}",
                        variant.name()
                    ),
                }
            }
        }
    }
}
