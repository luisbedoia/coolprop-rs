//! Runtime checks of the build: the library links, CoolProp decodes the slim
//! CBOR catalog, every curated fluid opens with metadata matching
//! `FluidData`, and fluids left out of the catalog are really gone.

use std::ffi::{CStr, CString, c_char, c_long};
use std::sync::{Mutex, MutexGuard};

use coolprop_sys::catalog::{FLUIDS, Variant};

/// CoolProp's global state is not thread-safe; tests run in parallel.
fn lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const BUF: usize = 1024;

fn global_param(name: &str) -> String {
    let name = CString::new(name).unwrap();
    let mut buf = [0 as c_char; BUF];
    // SAFETY: `name` is NUL-terminated and `buf` holds `BUF` bytes.
    let ret =
        unsafe { coolprop_sys::get_global_param_string(name.as_ptr(), buf.as_mut_ptr(), BUF as _) };
    assert_eq!(ret, 1, "get_global_param_string({name:?}) failed");
    // SAFETY: CoolProp NUL-terminates the output within `BUF`.
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

fn param_index(name: &str) -> c_long {
    let name = CString::new(name).unwrap();
    // SAFETY: `name` is NUL-terminated.
    unsafe { coolprop_sys::get_param_index(name.as_ptr()) }
}

/// Owned `AbstractState` handle.
struct State(c_long);

impl State {
    fn open(backend: &str, fluids: &str) -> Result<Self, String> {
        let backend = CString::new(backend).unwrap();
        let fluids = CString::new(fluids).unwrap();
        let mut err: c_long = 0;
        let mut buf = [0 as c_char; BUF];
        // SAFETY: inputs are NUL-terminated and `buf` holds `BUF` bytes.
        let handle = unsafe {
            coolprop_sys::AbstractState_factory(
                backend.as_ptr(),
                fluids.as_ptr(),
                &mut err,
                buf.as_mut_ptr(),
                BUF as _,
            )
        };
        if err == 0 {
            Ok(Self(handle))
        } else {
            // SAFETY: CoolProp NUL-terminates the message within `BUF`.
            Err(unsafe { CStr::from_ptr(buf.as_ptr()) }
                .to_string_lossy()
                .into_owned())
        }
    }

    fn keyed_output(&self, param: &str) -> f64 {
        let mut err: c_long = 0;
        let mut buf = [0 as c_char; BUF];
        // SAFETY: `self.0` is a live handle and `buf` holds `BUF` bytes.
        let value = unsafe {
            coolprop_sys::AbstractState_keyed_output(
                self.0,
                param_index(param),
                &mut err,
                buf.as_mut_ptr(),
                BUF as _,
            )
        };
        assert_eq!(err, 0, "keyed_output({param}) failed");
        value
    }
}

impl Drop for State {
    fn drop(&mut self) {
        let mut err: c_long = 0;
        let mut buf = [0 as c_char; BUF];
        // SAFETY: `self.0` is a live handle, freed exactly once.
        unsafe { coolprop_sys::AbstractState_free(self.0, &mut err, buf.as_mut_ptr(), BUF as _) };
    }
}

fn rel_diff(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-30)
}

#[test]
fn version_is_reported() {
    let _g = lock();
    let version = global_param("version");
    assert!(
        version.starts_with("8."),
        "unexpected CoolProp version {version}"
    );
}

#[test]
fn catalog_matches_variants() {
    assert_eq!(Variant::ALL.len(), FLUIDS.len());
    for (i, &v) in Variant::ALL.iter().enumerate() {
        assert_eq!(v as usize, i);
        assert_eq!(v.name(), FLUIDS[i].name);
        assert_eq!(Variant::from_name(v.name()), Some(v));
    }
}

/// Molar mass is a passthrough of the JSON, so it must match exactly. The
/// critical point is recomputed by each EOS and differs from the published
/// (often rounded) value by up to ~1.2e-3 (CarbonMonoxide pcrit: 3.4982 MPa
/// from the EOS vs 3.494 MPa in `STATES.critical`).
#[test]
fn every_curated_fluid_opens_with_matching_metadata() {
    let _g = lock();
    for data in FLUIDS {
        let state = State::open("HEOS", data.name)
            .unwrap_or_else(|e| panic!("{} does not open: {e}", data.name));
        for (param, runtime, published, tol) in [
            (
                "molar_mass",
                state.keyed_output("molar_mass"),
                data.molar_mass,
                1e-12,
            ),
            ("Tcrit", state.keyed_output("Tcrit"), data.t_critical, 2e-3),
            ("pcrit", state.keyed_output("pcrit"), data.p_critical, 2e-3),
        ] {
            let diff = rel_diff(runtime, published);
            assert!(
                diff < tol,
                "{}: {param} runtime {runtime} vs catalog {published} (rel diff {diff:e})",
                data.name
            );
        }
    }
}

#[test]
fn non_curated_fluid_is_absent() {
    let _g = lock();
    // Any CoolProp fluid outside the catalog will do; the list only needs one
    // that no COOLPROP_FLUIDS selection in this repo includes.
    let excluded = ["R22", "Krypton", "Neon", "Xenon", "R11"]
        .into_iter()
        .find(|name| Variant::from_name(name).is_none())
        .expect("every candidate is curated; extend the list");
    assert!(State::open("HEOS", excluded).is_err());
}

/// Exercises the filtered binary-interaction-parameter header.
#[test]
fn curated_mixture_opens() {
    let _g = lock();
    if Variant::from_name("Methane").is_none() || Variant::from_name("Ethane").is_none() {
        return; // custom COOLPROP_FLUIDS without this pair
    }
    State::open("HEOS", "Methane&Ethane").expect("Methane&Ethane opens");
}
