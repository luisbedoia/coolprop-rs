//! Reference states: where enthalpy and entropy are measured from. A
//! session opened with one has the textbook values at its defining state,
//! keeps every other property and every difference, and leaves the other
//! sessions (open or opened later) alone.

use coolprop::{Fluid, Input, PropsError, Reference, State, Variant};

const T0: f64 = 273.15;

fn saturated_liquid_at(f: &Fluid, t: f64) -> State {
    f.state(Input::Temperature(t), Input::Quality(0.0)).unwrap()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn each_reference_has_its_defining_values() {
    let ashrae = Fluid::with_reference(Variant::R134a, Reference::Ashrae).unwrap();
    let s = saturated_liquid_at(&ashrae, T0 - 40.0);
    assert!(close(s.enthalpy(), 0.0, 1e-6), "h = {}", s.enthalpy());
    assert!(close(s.entropy(), 0.0, 1e-9), "s = {}", s.entropy());

    let iir = Fluid::with_reference(Variant::R134a, Reference::Iir).unwrap();
    let s = saturated_liquid_at(&iir, T0);
    assert!(close(s.enthalpy(), 200e3, 1e-6));
    assert!(close(s.entropy(), 1e3, 1e-9));

    let nbp = Fluid::with_reference(Variant::R134a, Reference::Nbp).unwrap();
    let s = nbp
        .state(Input::Pressure(101_325.0), Input::Quality(0.0))
        .unwrap();
    assert!(close(s.enthalpy(), 0.0, 1e-6));
    assert!(close(s.entropy(), 0.0, 1e-9));

    assert_eq!(ashrae.reference(), Reference::Ashrae);
    assert_eq!(
        Fluid::new(Variant::R134a).unwrap().reference(),
        Reference::Default
    );
}

#[test]
fn only_absolute_enthalpy_and_entropy_change() {
    let default = Fluid::new(Variant::R134a).unwrap();
    let ashrae = Fluid::with_reference(Variant::R134a, Reference::Ashrae).unwrap();
    let inputs = [
        (Input::Pressure(1e6), Input::Temperature(T0 + 50.0)),
        (Input::Pressure(2e5), Input::Quality(0.3)),
        (Input::Temperature(T0 - 20.0), Input::Pressure(5e5)),
    ];
    let pairs: Vec<(State, State)> = inputs
        .iter()
        .map(|&(a, b)| (default.state(a, b).unwrap(), ashrae.state(a, b).unwrap()))
        .collect();
    let (dh, ds) = (
        pairs[0].0.enthalpy() - pairs[0].1.enthalpy(),
        pairs[0].0.entropy() - pairs[0].1.entropy(),
    );
    assert!(dh.abs() > 1e3, "the references differ");
    for (d, a) in &pairs {
        // One offset each, for every state; u moves with h.
        assert!(close(d.enthalpy() - a.enthalpy(), dh, 1e-6));
        assert!(close(d.entropy() - a.entropy(), ds, 1e-9));
        assert!(close(d.internal_energy() - a.internal_energy(), dh, 1e-6));
        assert_eq!(d.temperature(), a.temperature());
        assert_eq!(d.pressure(), a.pressure());
        assert!(close(d.density(), a.density(), 1e-9 * d.density()));
        assert_eq!(d.quality(), a.quality());
    }
    // Inputs are read in the session's reference too.
    let back = ashrae
        .state(Input::Pressure(1e6), Input::Enthalpy(pairs[0].1.enthalpy()))
        .unwrap();
    assert!(close(back.temperature(), T0 + 50.0, 1e-6));
}

#[test]
fn other_sessions_keep_theirs() {
    let before = Fluid::new(Variant::R134a).unwrap();
    let h_before = saturated_liquid_at(&before, T0).enthalpy();
    let _ashrae = Fluid::with_reference(Variant::R134a, Reference::Ashrae).unwrap();
    // Open before, and opened after, with the default reference.
    assert_eq!(saturated_liquid_at(&before, T0).enthalpy(), h_before);
    let after = Fluid::new(Variant::R134a).unwrap();
    assert_eq!(saturated_liquid_at(&after, T0).enthalpy(), h_before);
}

#[test]
fn a_reference_below_the_triple_point_is_refused() {
    // Water's triple point (0.01 °C) is above −40 °C and 0 °C.
    for reference in [Reference::Ashrae, Reference::Iir] {
        let err = Fluid::with_reference(Variant::Water, reference).unwrap_err();
        assert!(
            matches!(&err, PropsError::InvalidInput(m) if m.contains(reference.name())),
            "{err}"
        );
    }
    // NBP is fine: water boils at 1 atm well above its triple point.
    let nbp = Fluid::with_reference(Variant::Water, Reference::Nbp).unwrap();
    let s = nbp
        .state(Input::Pressure(101_325.0), Input::Quality(0.0))
        .unwrap();
    assert!(close(s.enthalpy(), 0.0, 1e-6));
}

#[test]
fn pseudo_pure_mixtures_take_one_too() {
    // Air boils at 1 atm (its bubble point) near 79 K: NBP applies.
    let air = Fluid::with_reference(Variant::Air, Reference::Nbp).unwrap();
    let s = air
        .state(Input::Pressure(101_325.0), Input::Quality(0.0))
        .unwrap();
    assert!(close(s.enthalpy(), 0.0, 1e-6), "h = {}", s.enthalpy());
    assert!(close(s.entropy(), 0.0, 1e-9), "s = {}", s.entropy());
    let default = Fluid::new(Variant::Air).unwrap();
    let (a, b) = (Input::Pressure(1e5), Input::Temperature(300.0));
    let (gas, gas0) = (air.state(a, b).unwrap(), default.state(a, b).unwrap());
    assert!(close(gas.density(), gas0.density(), 1e-9 * gas0.density()));
    // At −40 °C air is supercritical: it has no saturated liquid there.
    assert!(Fluid::with_reference(Variant::Air, Reference::Ashrae).is_err());
}

#[test]
fn names_round_trip() {
    for r in Reference::ALL {
        assert_eq!(Reference::from_name(r.name()), Some(r));
    }
    assert_eq!(Reference::from_name("ashrae"), None);
}
