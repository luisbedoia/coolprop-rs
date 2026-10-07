//! Tests for the derived/transport properties exposed on `State`
//! (cp, cv, viscosity, conductivity, Prandtl, Gibbs, Z, speed of sound).
//!
//! In a single-phase state they must all resolve to finite values. In the
//! two-phase region some of them are not always defined, so `solve` must
//! still succeed and each derived property must be either a finite value or
//! `None` — never a NaN and never a hard failure.

use coolprop::{Fluid, Input, Variant};

#[test]
fn single_phase_water_has_all_derived_properties() {
    let f = Fluid::new(Variant::Water).unwrap();
    // Subcooled liquid water at 1 atm, 25 °C.
    let s = f
        .state(Input::Pressure(101_325.0), Input::Temperature(298.15))
        .unwrap();

    let finite = |label: &str, v: Option<f64>| {
        let v = v.unwrap_or_else(|| panic!("{label} should be Some for single-phase water"));
        assert!(v.is_finite(), "{label} should be finite, got {v}");
    };

    finite("cp", s.cp());
    finite("cv", s.cv());
    finite("viscosity", s.viscosity());
    finite("conductivity", s.conductivity());
    finite("prandtl", s.prandtl());
    finite("gibbs", s.gibbs());
    finite("compressibility", s.compressibility());
    finite("speed_of_sound", s.speed_of_sound());

    // Sanity: cp > cv > 0 for a liquid, Z > 0.
    assert!(s.cp().unwrap() > s.cv().unwrap());
    assert!(s.cv().unwrap() > 0.0);
    assert!(s.compressibility().unwrap() > 0.0);
}

#[test]
fn single_phase_properties_are_undefined_inside_the_dome() {
    let f = Fluid::new(Variant::Water).unwrap();
    // Wet steam at 1 atm, x = 0.5: CoolProp still returns numbers for cp,
    // viscosity, Z, … there, which mean nothing for a liquid–vapor mixture.
    let s = f
        .state(Input::Pressure(101_325.0), Input::Quality(0.5))
        .unwrap();
    assert_eq!(s.quality(), Some(0.5));
    assert!(s.enthalpy().is_finite());
    assert_eq!(s.cp(), None, "cp");
    assert_eq!(s.cv(), None, "cv");
    assert_eq!(s.viscosity(), None, "viscosity");
    assert_eq!(s.conductivity(), None, "conductivity");
    assert_eq!(s.prandtl(), None, "prandtl");
    assert_eq!(s.compressibility(), None, "compressibility");
    assert_eq!(s.speed_of_sound(), None, "speed of sound");
    // g is the same for both phases at saturation: defined.
    assert!(s.gibbs().unwrap().is_finite());
}

/// On the dome itself the single-phase properties are the saturated
/// phase's: the same as just off the dome.
#[test]
fn saturated_states_keep_their_phase_properties() {
    let f = Fluid::new(Variant::Water).unwrap();
    let p = 101_325.0;
    let liquid = f.state(Input::Pressure(p), Input::Quality(0.0)).unwrap();
    let vapor = f.state(Input::Pressure(p), Input::Quality(1.0)).unwrap();
    let t_sat = liquid.temperature();
    let below = f
        .state(Input::Pressure(p), Input::Temperature(t_sat - 0.01))
        .unwrap();
    let above = f
        .state(Input::Pressure(p), Input::Temperature(t_sat + 0.01))
        .unwrap();
    let close = |a: Option<f64>, b: Option<f64>, label: &str| {
        let (a, b) = (a.expect(label), b.expect(label));
        assert!((a / b - 1.0).abs() < 1e-3, "{label}: {a} vs {b}");
    };
    close(liquid.cp(), below.cp(), "liquid cp");
    close(liquid.viscosity(), below.viscosity(), "liquid viscosity");
    close(vapor.cp(), above.cp(), "vapor cp");
    close(
        vapor.conductivity(),
        above.conductivity(),
        "vapor conductivity",
    );
}
