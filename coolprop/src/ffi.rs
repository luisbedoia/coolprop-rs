//! The only module that calls into `coolprop-sys`. Every C-API call goes
//! through [`call`], which owns the errcode/message-buffer protocol, and runs
//! under [`coolprop_lock`].

use std::ffi::{CStr, CString, c_char, c_long};
use std::sync::{Mutex, MutexGuard};

use crate::error::PropsError;
use crate::input::{Input, PAIRS};
use crate::phase::Phase;
use crate::property::Property;

const ERR_BUF_LEN: usize = 1024;

/// CoolProp's global state (library load, handle manager, `errstring`) is
/// not safe to share across threads, so all C-API calls are serialized.
fn coolprop_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// CoolProp version string (e.g. `"8.0.0"`).
pub fn version() -> String {
    let _guard = coolprop_lock();
    get_global_param_string("version", 256).unwrap_or_else(|| String::from("unknown"))
}

/// Caller must hold `coolprop_lock()`.
fn last_errstring() -> String {
    get_global_param_string("errstring", ERR_BUF_LEN)
        .unwrap_or_else(|| String::from("unknown error"))
}

/// Caller must hold `coolprop_lock()`.
fn get_global_param_string(name: &str, buf_size: usize) -> Option<String> {
    let param = CString::new(name).ok()?;
    let mut buf = vec![0 as c_char; buf_size];
    // SAFETY: `param` is NUL-terminated and `buf` holds `buf_size` bytes.
    let rc = unsafe {
        coolprop_sys::get_global_param_string(param.as_ptr(), buf.as_mut_ptr(), buf_size as _)
    };
    (rc != 0).then(|| c_buf_to_string(&buf))
}

/// Runs one C-API call that reports failure through `(errcode, message,
/// buffer_length)` out-parameters, mapping a non-zero errcode to
/// [`PropsError::CoolProp`]. Caller must hold `coolprop_lock()`.
fn call<T>(f: impl FnOnce(*mut c_long, *mut c_char, c_long) -> T) -> Result<T, PropsError> {
    let mut errcode: c_long = 0;
    let mut msg = [0 as c_char; ERR_BUF_LEN];
    let value = f(&mut errcode, msg.as_mut_ptr(), ERR_BUF_LEN as c_long);
    if errcode != 0 {
        let msg = c_buf_to_string(&msg);
        return Err(PropsError::CoolProp(if msg.is_empty() {
            last_errstring()
        } else {
            msg
        }));
    }
    Ok(value)
}

/// Like [`call`], for calls returning a value that must be finite.
fn call_finite(f: impl FnOnce(*mut c_long, *mut c_char, c_long) -> f64) -> Result<f64, PropsError> {
    let value = call(f)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(PropsError::CoolProp(last_errstring()))
    }
}

/// Decodes a NUL-terminated C buffer; empty if no NUL is found.
fn c_buf_to_string(buf: &[c_char]) -> String {
    // SAFETY: `c_char` and `u8` have the same size and alignment.
    let bytes = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), buf.len()) };
    CStr::from_bytes_until_nul(bytes)
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(crate) struct StateValues {
    pub(crate) pressure: f64,
    pub(crate) temperature: f64,
    pub(crate) density: f64,
    pub(crate) enthalpy: f64,
    pub(crate) entropy: f64,
    pub(crate) internal_energy: f64,
    pub(crate) quality: Option<f64>,
    pub(crate) phase: Option<Phase>,
    pub(crate) cp: Option<f64>,
    pub(crate) cv: Option<f64>,
    pub(crate) viscosity: Option<f64>,
    pub(crate) conductivity: Option<f64>,
    pub(crate) prandtl: Option<f64>,
    pub(crate) gibbs: Option<f64>,
    pub(crate) compressibility: Option<f64>,
    pub(crate) speed_of_sound: Option<f64>,
}

/// Owned CoolProp `AbstractState` handle, freed on drop.
#[derive(Debug)]
pub(crate) struct AbstractStateHandle {
    handle: c_long,
}

impl AbstractStateHandle {
    pub(crate) fn new(backend: &str, fluids: &str) -> Result<Self, PropsError> {
        let c_backend = CString::new(backend)?;
        let c_fluids = CString::new(fluids)?;
        let _guard = coolprop_lock();
        let handle = call(|err, msg, len| {
            // SAFETY: inputs are NUL-terminated; `call` provides valid
            // errcode/message out-parameters.
            unsafe {
                coolprop_sys::AbstractState_factory(
                    c_backend.as_ptr(),
                    c_fluids.as_ptr(),
                    err,
                    msg,
                    len,
                )
            }
        })?;
        if handle < 0 {
            return Err(PropsError::CoolProp(last_errstring()));
        }
        Ok(Self { handle })
    }

    /// Caller must hold the global lock.
    fn update(&self, in1: Input, in2: Input) -> Result<(), PropsError> {
        let (pair_name, v1, v2) = resolve_pair(in1, in2)?;
        let pair = input_pair_index(pair_name)?;
        call(|err, msg, len| {
            // SAFETY: `self.handle` is live; `call` provides the out-parameters.
            unsafe { coolprop_sys::AbstractState_update(self.handle, pair, v1, v2, err, msg, len) }
        })
    }

    /// Caller must hold the global lock.
    fn keyed_output(&self, prop: Property) -> Result<f64, PropsError> {
        self.keyed_output_by_name_locked(prop.as_str())
    }

    /// Reads a parameter by CoolProp name. Trivial constants (Tcrit, M, …)
    /// need no prior `update`; state-dependent ones (P, T, H, …) do.
    pub(crate) fn keyed_output_by_name(&self, name: &str) -> Result<f64, PropsError> {
        let _guard = coolprop_lock();
        self.keyed_output_by_name_locked(name)
    }

    fn keyed_output_by_name_locked(&self, name: &str) -> Result<f64, PropsError> {
        let param = param_index(name)?;
        call_finite(|err, msg, len| {
            // SAFETY: `self.handle` is live; `call` provides the out-parameters.
            unsafe { coolprop_sys::AbstractState_keyed_output(self.handle, param, err, msg, len) }
        })
    }

    /// Hot path for plotting: `update`s and returns only the two requested
    /// outputs, skipping the eager full-state reads `solve_state` does.
    #[allow(dead_code)] // used by `plot`, ported next
    pub(crate) fn solve_xy(
        &self,
        in1: Input,
        in2: Input,
        out1: Property,
        out2: Property,
    ) -> Result<(f64, f64), PropsError> {
        let _guard = coolprop_lock();
        self.update(in1, in2)?;
        Ok((self.keyed_output(out1)?, self.keyed_output(out2)?))
    }

    /// `update`s and returns all derived properties. Leaves the handle in
    /// the new state.
    pub(crate) fn solve_state(&self, in1: Input, in2: Input) -> Result<StateValues, PropsError> {
        let _guard = coolprop_lock();
        self.update(in1, in2)?;

        // Quality is only valid in the two-phase region; outside it CoolProp
        // returns a negative sentinel or fails.
        let quality = self
            .keyed_output(Property::Quality)
            .ok()
            .filter(|q| (0.0..=1.0).contains(q));
        let phase = self
            .keyed_output(Property::Phase)
            .ok()
            .and_then(Phase::from_index);
        // Best effort: some are undefined for some states (e.g. two-phase).
        let optional = |prop| self.keyed_output(prop).ok();

        Ok(StateValues {
            pressure: self.keyed_output(Property::Pressure)?,
            temperature: self.keyed_output(Property::Temperature)?,
            density: self.keyed_output(Property::Density)?,
            enthalpy: self.keyed_output(Property::Enthalpy)?,
            entropy: self.keyed_output(Property::Entropy)?,
            internal_energy: self.keyed_output(Property::InternalEnergy)?,
            quality,
            phase,
            cp: optional(Property::Cp),
            cv: optional(Property::Cv),
            viscosity: optional(Property::Viscosity),
            conductivity: optional(Property::Conductivity),
            prandtl: optional(Property::Prandtl),
            gibbs: optional(Property::GibbsEnergy),
            compressibility: optional(Property::Compressibility),
            speed_of_sound: optional(Property::SpeedOfSound),
        })
    }
}

impl Drop for AbstractStateHandle {
    fn drop(&mut self) {
        let _guard = coolprop_lock();
        // Nothing useful to do with a failure while dropping.
        let _ = call(|err, msg, len| {
            // SAFETY: `self.handle` is live and freed exactly once, here.
            unsafe { coolprop_sys::AbstractState_free(self.handle, err, msg, len) }
        });
    }
}

fn input_pair_index(name: &str) -> Result<c_long, PropsError> {
    let c_name = CString::new(name)?;
    // SAFETY: `c_name` is NUL-terminated.
    let idx = unsafe { coolprop_sys::get_input_pair_index(c_name.as_ptr()) };
    if idx < 0 {
        Err(PropsError::CoolProp(format!("unknown input pair: {name}")))
    } else {
        Ok(idx)
    }
}

fn param_index(name: &str) -> Result<c_long, PropsError> {
    let c_name = CString::new(name)?;
    // SAFETY: `c_name` is NUL-terminated.
    let idx = unsafe { coolprop_sys::get_param_index(c_name.as_ptr()) };
    if idx < 0 {
        Err(PropsError::CoolProp(format!("unknown parameter: {name}")))
    } else {
        Ok(idx)
    }
}

/// Maps the two inputs to CoolProp's pair name (`PT_INPUTS`, …), reordering
/// the values to the pair's signature.
fn resolve_pair(in1: Input, in2: Input) -> Result<(&'static str, f64, f64), PropsError> {
    let (k1, k2) = (in1.kind(), in2.kind());
    PAIRS
        .iter()
        .find_map(|&(a, b, name)| {
            if (a, b) == (k1, k2) {
                Some((name, in1.value(), in2.value()))
            } else if (a, b) == (k2, k1) {
                Some((name, in2.value(), in1.value()))
            } else {
                None
            }
        })
        .ok_or_else(|| {
            PropsError::InvalidInput(format!(
                "unsupported input pair: ({}, {})",
                k1.name(),
                k2.name()
            ))
        })
}
