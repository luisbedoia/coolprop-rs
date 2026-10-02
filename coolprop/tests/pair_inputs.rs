//! Round-trip every supported input pair, in both orders, through
//! `Fluid::state`: take a reference state, re-solve it from two of its own
//! properties and check that all six base properties come back.
//!
//! - Subcooled water (101 325 Pa, 298.15 K) for the 11 pairs without quality.
//! - Saturated water (101 325 Pa, x = 0.5) for the 3 pairs with quality.

use coolprop::{Fluid, Input, PropsError, State, Variant};

const TOL: f64 = 1e-4;

fn water() -> Fluid {
    Fluid::new(Variant::Water).unwrap()
}

#[track_caller]
fn assert_same_state(label: &str, got: &State, want: &State) {
    let close = |a: f64, b: f64| (a - b).abs() / b.abs().max(1.0) < TOL;
    for (name, g, w) in [
        ("P", got.pressure(), want.pressure()),
        ("T", got.temperature(), want.temperature()),
        ("D", got.density(), want.density()),
        ("H", got.enthalpy(), want.enthalpy()),
        ("S", got.entropy(), want.entropy()),
        ("U", got.internal_energy(), want.internal_energy()),
    ] {
        assert!(close(g, w), "{label}: {name} {g} vs {w}");
    }
}

/// Re-solves `reference` from each pair, as given and swapped.
fn round_trip(fluid: &Fluid, reference: &State, pairs: &[(Input, Input)]) {
    for &(a, b) in pairs {
        for (in1, in2) in [(a, b), (b, a)] {
            let label = format!("({in1:?}, {in2:?})");
            let s = fluid
                .state(in1, in2)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_same_state(&label, &s, reference);
        }
    }
}

#[test]
fn single_phase_pairs() {
    let f = water();
    let r = f
        .state(Input::Pressure(101_325.0), Input::Temperature(298.15))
        .unwrap();
    let (p, t, d) = (r.pressure(), r.temperature(), r.density());
    let (h, s, u) = (r.enthalpy(), r.entropy(), r.internal_energy());
    use Input::*;
    round_trip(
        &f,
        &r,
        &[
            (Pressure(p), Temperature(t)),
            (Pressure(p), Density(d)),
            (Pressure(p), Enthalpy(h)),
            (Pressure(p), Entropy(s)),
            (Pressure(p), InternalEnergy(u)),
            (Temperature(t), Density(d)),
            (Temperature(t), Entropy(s)),
            (Density(d), Enthalpy(h)),
            (Density(d), Entropy(s)),
            (Density(d), InternalEnergy(u)),
            (Enthalpy(h), Entropy(s)),
        ],
    );
}

#[test]
fn two_phase_pairs() {
    let f = water();
    let r = f
        .state(Input::Pressure(101_325.0), Input::Quality(0.5))
        .unwrap();
    let x = r.quality().expect("two-phase reference has a quality");
    let (p, t, d) = (r.pressure(), r.temperature(), r.density());
    use Input::*;
    round_trip(
        &f,
        &r,
        &[
            (Pressure(p), Quality(x)),
            (Quality(x), Temperature(t)),
            (Density(d), Quality(x)),
        ],
    );
}

#[test]
fn unsupported_pair_is_invalid_input() {
    let r = water().state(Input::Quality(0.5), Input::InternalEnergy(2.5e5));
    assert!(matches!(r, Err(PropsError::InvalidInput(_))), "{r:?}");
}

#[test]
fn same_property_twice_is_invalid_input() {
    let r = water().state(Input::Pressure(1.0e5), Input::Pressure(2.0e5));
    assert!(matches!(r, Err(PropsError::InvalidInput(_))), "{r:?}");
}

#[test]
fn out_of_range_state_is_coolprop_error() {
    let r = water().state(Input::Pressure(101_325.0), Input::Temperature(-10.0));
    assert!(matches!(r, Err(PropsError::CoolProp(_))), "{r:?}");
}
