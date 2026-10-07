//! A `Fluid` session never fails, nor gives other values, because of what
//! was solved before with it. CoolProp's `AbstractState` kept the phase of
//! its last flash, which made later pressure–temperature flashes fail or,
//! worse, silently land on wrong roots.

use coolprop::schema;
use coolprop::{Fluid, Input, InputKind, State, Variant};

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

/// One input pair taken from a reference state, and what solving it should
/// give: a fresh session's result or, where a fresh session fails, the
/// reference state itself.
struct Case {
    a: Input,
    b: Input,
    /// (ρ, h) to expect from any successful solve.
    expected: (f64, f64),
    fresh_solves: bool,
}

#[test]
fn a_session_never_fails_nor_misleads_where_a_fresh_one_would_not() {
    let names = ["Water", "R134a", "CarbonDioxide", "R410A"];
    let mut tested = 0;
    for variant in names.iter().filter_map(|n| Variant::from_name(n)) {
        tested += 1;
        let fluid = Fluid::new(variant).unwrap();
        let mut cases = Vec::new();
        for s in reference_states(&fluid) {
            for (a, b) in schema::pairs() {
                let (Some(va), Some(vb)) = (value_of(&s, a), value_of(&s, b)) else {
                    continue;
                };
                let (ia, ib) = (a.with_value(va), b.with_value(vb));
                let fresh = Fluid::new(variant).unwrap().state(ia, ib);
                cases.push(Case {
                    a: ia,
                    b: ib,
                    expected: fresh
                        .as_ref()
                        .map_or((s.density(), s.enthalpy()), |f| (f.density(), f.enthalpy())),
                    fresh_solves: fresh.is_ok(),
                });
            }
        }
        assert!(cases.len() > 60, "{} cases", cases.len());

        // Every ordered couple (A, B), B right after A in one session (a
        // flash in between could clear a stale phase). B may fail only where
        // a fresh session fails too; it may succeed where a fresh session
        // fails (CoolProp starts some flashes from the previous state, which
        // helps blends converge), but never with other values.
        let shared = Fluid::new(variant).unwrap();
        for first in &cases {
            for case in &cases {
                let _ = shared.state(first.a, first.b);
                let context = || {
                    format!(
                        "{}: {:?}, {:?} after {:?}, {:?}",
                        variant.name(),
                        case.a,
                        case.b,
                        first.a,
                        first.b
                    )
                };
                match shared.state(case.a, case.b) {
                    Ok(got) => {
                        let (d, h) = (got.density(), got.enthalpy());
                        let (de, he) = case.expected;
                        let rel = |x: f64, y: f64| (x - y).abs() / y.abs().max(1.0);
                        assert!(
                            rel(d, de) < 1e-6 && rel(h, he) < 1e-6,
                            "{} gave ρ={d}, h={h}; expected ρ={de}, h={he}",
                            context()
                        );
                    }
                    Err(e) => assert!(
                        !case.fresh_solves,
                        "{} failed ({e}) where a fresh session solves it",
                        context()
                    ),
                }
            }
        }
    }
    assert!(tested >= 3, "only {tested} of the fluids are in this build");
}
