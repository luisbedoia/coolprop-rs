use coolprop_sys::catalog::{FluidData, Variant};

use crate::error::PropsError;
use crate::ffi::AbstractStateHandle;
use crate::input::Input;
use crate::state::State;

/// Critical point as computed by the fluid's equation of state, in SI.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CriticalPoint {
    /// K
    pub temperature: f64,
    /// Pa
    pub pressure: f64,
    /// kg/m³
    pub density: f64,
}

/// Working session for one curated fluid: owns a CoolProp `AbstractState`
/// (HEOS backend) and frees it on drop.
///
/// ```no_run
/// use coolprop::{Fluid, Input, Variant};
/// let water = Fluid::new(Variant::Water)?;
/// let s = water.state(Input::Pressure(101_325.0), Input::Temperature(298.15))?;
/// println!("h = {} J/kg, Tc = {} K", s.enthalpy(), water.critical().temperature);
/// # Ok::<(), coolprop::PropsError>(())
/// ```
#[derive(Debug)]
pub struct Fluid {
    variant: Variant,
    critical: CriticalPoint,
    handle: AbstractStateHandle,
}

impl Fluid {
    /// Opens a session for a curated fluid.
    pub fn new(variant: Variant) -> Result<Self, PropsError> {
        let handle = AbstractStateHandle::new("HEOS", variant.name())?;
        let critical = CriticalPoint {
            temperature: handle.keyed_output_by_name("Tcrit")?,
            pressure: handle.keyed_output_by_name("pcrit")?,
            density: handle.keyed_output_by_name("rhocrit")?,
        };
        Ok(Self {
            variant,
            critical,
            handle,
        })
    }

    /// Opens a session by canonical name or alias (`"Water"`, `"H2O"`, …).
    pub fn from_name(name: &str) -> Result<Self, PropsError> {
        let variant =
            Variant::from_name(name).ok_or_else(|| PropsError::UnknownFluid(name.to_owned()))?;
        Self::new(variant)
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    /// Catalog constants (identity, molar mass, triple point, EOS limits).
    pub fn data(&self) -> &'static FluidData {
        self.variant.data()
    }

    /// Critical point of the EOS (read once, when the session was opened).
    pub fn critical(&self) -> CriticalPoint {
        self.critical
    }

    /// Solves the state fixed by two independent inputs, in either order.
    pub fn state(&self, in1: Input, in2: Input) -> Result<State, PropsError> {
        State::solve(&self.handle, in1, in2)
    }
}
