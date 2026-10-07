//! `Fluid` metadata: catalog constants, the EOS critical point and lookup by
//! name. Reference values: NIST WebBook.

use coolprop::{Fluid, PropsError, Variant};

#[track_caller]
fn assert_close(actual: f64, expected: f64, rel_tol: f64, label: &str) {
    let rel_err = (actual - expected).abs() / expected.abs();
    assert!(
        rel_err <= rel_tol,
        "{label}: {actual} vs {expected} (rel err {rel_err:.3e}, tol {rel_tol:.1e})"
    );
}

#[test]
fn water_constants() {
    let f = Fluid::new(Variant::Water).unwrap();
    let c = f.critical();
    assert_close(c.temperature, 647.096, 1e-6, "Tcrit");
    assert_close(c.pressure, 22.064e6, 1e-6, "pcrit");
    assert_close(c.density, 322.0, 1e-6, "rhocrit");

    let d = f.data();
    assert_eq!(d.name, "Water");
    assert_eq!(d.cas, "7732-18-5");
    assert_close(d.molar_mass, 0.018_015_268, 1e-9, "molar mass");
    assert_close(d.t_triple, 273.16, 1e-9, "Ttriple");
    assert_close(d.p_triple, 611.655, 1e-5, "ptriple");
}

#[test]
fn critical_point_comes_from_the_eos() {
    // Oxygen's published critical density (436.14 kg/m³) is 2 % above the
    // value its EOS uses; `critical()` must report the EOS one.
    let f = Fluid::new(Variant::Oxygen).unwrap();
    assert_close(f.critical().density, 426.934, 1e-5, "O2 rhocrit");
}

#[test]
fn from_name_accepts_aliases() {
    let f = Fluid::from_name("H2O").unwrap();
    assert_eq!(f.variant(), Variant::Water);
    assert_eq!(Fluid::from_name("water").unwrap().variant(), Variant::Water);
}

#[test]
fn from_name_rejects_unknown_fluid() {
    let r = Fluid::from_name("Unobtainium");
    assert!(matches!(r, Err(PropsError::UnknownFluid(_))), "{r:?}");
}

/// Air is a pseudo-pure mixture: a catalog flag says so, and inside the
/// dome only states that agree with CoolProp's own definition (by pressure
/// and quality) are returned. From pairs without pressure CoolProp's
/// solvers fail or, worse, land on other states.
#[test]
fn pseudo_pure_mixtures_are_flagged_and_never_give_wrong_two_phase_states() {
    use coolprop::{Fluid, Input, Variant};
    assert!(!Variant::Water.data().pseudo_pure);
    let air = Fluid::new(Variant::from_name("Air").unwrap()).unwrap();
    assert!(air.data().pseudo_pure);

    let wet = air
        .state(Input::Pressure(200_000.0), Input::Quality(0.4))
        .unwrap();
    let (t, rho, h, s, u) = (
        wet.temperature(),
        wet.density(),
        wet.enthalpy(),
        wet.entropy(),
        wet.internal_energy(),
    );
    let same = |r: &coolprop::State| {
        (r.enthalpy() - h).abs() < 1e-6 * h.abs().max(1.0) && (r.density() / rho - 1.0).abs() < 1e-6
    };
    for (a, b) in [
        (Input::Temperature(t), Input::Density(rho)),
        (Input::Temperature(t), Input::Entropy(s)),
        (Input::Enthalpy(h), Input::Entropy(s)),
        (Input::Density(rho), Input::Quality(0.4)),
        (Input::Density(rho), Input::Enthalpy(h)),
        (Input::Density(rho), Input::InternalEnergy(u)),
        (Input::Pressure(wet.pressure()), Input::Enthalpy(h)),
        (Input::Pressure(wet.pressure()), Input::Entropy(s)),
    ] {
        // In one session, after other states: a stale start must not help
        // a wrong answer through either.
        let _ = air.state(Input::Pressure(1e5), Input::Temperature(300.0));
        if let Ok(r) = air.state(a, b) {
            assert!(
                same(&r),
                "{a:?}, {b:?} gave h={} instead of {h}",
                r.enthalpy()
            );
        }
    }
    // Pairs with pressure are CoolProp's own definition: they must solve.
    assert!(
        air.state(Input::Pressure(wet.pressure()), Input::Enthalpy(h))
            .is_ok()
    );

    // Pure fluids keep every pair.
    let water = Fluid::new(Variant::Water).unwrap();
    let s = water
        .state(Input::Pressure(101_325.0), Input::Quality(0.4))
        .unwrap();
    let back = water
        .state(Input::Density(s.density()), Input::Quality(0.4))
        .unwrap();
    assert!((back.pressure() / s.pressure() - 1.0).abs() < 1e-6);
}
