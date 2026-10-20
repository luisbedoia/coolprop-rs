//! Safe Rust wrapper over the CoolProp thermophysical property library.
//!
//! Pick a fluid from the curated [`catalog`], open a [`Fluid`] session and
//! solve [`State`]s from two independent [`Input`]s. Everything is SI.
//!
//! ```no_run
//! use coolprop::{Fluid, Input, Variant};
//! let water = Fluid::new(Variant::Water)?;
//! let s = water.state(Input::Pressure(101_325.0), Input::Temperature(298.15))?;
//! let _h = s.enthalpy();
//! # Ok::<(), coolprop::PropsError>(())
//! ```

mod error;
mod ffi;
mod fluid;
mod fluid_names;
mod input;
mod locale;
mod phase;
pub mod plot;
mod property;
mod reference;
pub mod schema;
mod state;

pub use coolprop_sys::catalog;
pub use coolprop_sys::catalog::{FluidData, Variant};

pub use error::PropsError;
pub use ffi::version;
pub use fluid::{CriticalPoint, Fluid};
pub use input::{Input, InputKind};
pub use locale::Locale;
pub use phase::Phase;
pub use reference::Reference;
pub use state::State;
