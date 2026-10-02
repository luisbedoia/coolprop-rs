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
fn two_phase_water_solves_with_well_formed_derived_properties() {
    let f = Fluid::new(Variant::Water).unwrap();
    // Two-phase mixture at 1 atm, x = 0.5. solve must still succeed.
    let s = f
        .state(Input::Pressure(101_325.0), Input::Quality(0.5))
        .unwrap();

    // The core thermodynamic state is still fully defined.
    assert!(s.pressure().is_finite());
    assert!(s.enthalpy().is_finite());
    assert_eq!(s.quality(), Some(0.5));

    // Each derived property is either a finite value or None — the
    // best-effort read must never surface a NaN or panic the solve.
    let well_formed = |label: &str, v: Option<f64>| {
        if let Some(v) = v {
            assert!(v.is_finite(), "{label} should be finite when Some, got {v}");
        }
    };
    well_formed("cp", s.cp());
    well_formed("cv", s.cv());
    well_formed("viscosity", s.viscosity());
    well_formed("conductivity", s.conductivity());
    well_formed("prandtl", s.prandtl());
    well_formed("gibbs", s.gibbs());
    well_formed("compressibility", s.compressibility());
    well_formed("speed_of_sound", s.speed_of_sound());
}
