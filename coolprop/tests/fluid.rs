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
