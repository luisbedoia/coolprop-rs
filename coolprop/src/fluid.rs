use coolprop_sys::catalog::{FluidData, Variant};

use crate::error::PropsError;
use crate::ffi::AbstractStateHandle;
use crate::input::{Input, InputKind};
use crate::phase::Phase;
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
    ///
    /// A state at negative pressure is refused. A pseudo-pure mixture
    /// ([`FluidData::pseudo_pure`]) solved from a pair without pressure is
    /// also checked: CoolProp defines its phase boundaries by pressure, and
    /// from other pairs its solvers can land, without failing, on states that
    /// are not in equilibrium (for Air inside the dome: a two-phase state
    /// that does not give back the inputs, a gas above its dew pressure, a
    /// liquid at negative pressure; off by up to 160×). Such results are
    /// refused rather than returned.
    pub fn state(&self, in1: Input, in2: Input) -> Result<State, PropsError> {
        let state = State::solve(&self.handle, in1, in2)?;
        if state.pressure() <= 0.0 {
            return Err(PropsError::CoolProp(format!(
                "no state of {} at positive pressure has these inputs",
                self.data().name
            )));
        }
        let with_pressure = in1.kind() == InputKind::Pressure || in2.kind() == InputKind::Pressure;
        if !self.data().pseudo_pure || with_pressure || self.in_equilibrium(&state, in1, in2)? {
            return Ok(state);
        }
        Err(PropsError::CoolProp(format!(
            "{} is a pseudo-pure mixture: CoolProp cannot solve this state from {} and {}; \
             give the pressure and another property instead",
            self.data().name,
            in1.kind().name(),
            in2.kind().name()
        )))
    }

    /// Whether a pseudo-pure mixture's `state`, solved from `in1` and `in2`,
    /// is the equilibrium state CoolProp defines by pressure: inside the dome
    /// it must give back both inputs when solved from its pressure and
    /// quality; out of it, below the critical temperature, a gas must be at
    /// or below its dew pressure and a liquid at or above its bubble one.
    fn in_equilibrium(&self, state: &State, in1: Input, in2: Input) -> Result<bool, PropsError> {
        const TOL: f64 = 1e-6;
        if let Some(x) = state.quality().filter(|x| *x > 0.0 && *x < 1.0) {
            let canonical = State::solve(
                &self.handle,
                Input::Pressure(state.pressure()),
                Input::Quality(x),
            )?;
            let agrees = |input: Input| {
                input_value(&canonical, input.kind()).is_some_and(|v| {
                    (v - input.value()).abs() <= TOL * input.value().abs().max(1.0)
                })
            };
            return Ok(agrees(in1) && agrees(in2));
        }
        let t = state.temperature();
        if t >= self.critical.temperature {
            return Ok(true);
        }
        let saturation = |x: f64| -> Result<f64, PropsError> {
            Ok(State::solve(&self.handle, Input::Temperature(t), Input::Quality(x))?.pressure())
        };
        let p = state.pressure();
        Ok(match state.phase() {
            Some(Phase::Gas) => p <= saturation(1.0)? * (1.0 + TOL),
            Some(Phase::Liquid) => p >= saturation(0.0)? * (1.0 - TOL),
            _ => true,
        })
    }
}

/// The value of an input property in a solved state.
fn input_value(s: &State, kind: InputKind) -> Option<f64> {
    Some(match kind {
        InputKind::Pressure => s.pressure(),
        InputKind::Temperature => s.temperature(),
        InputKind::Density => s.density(),
        InputKind::Enthalpy => s.enthalpy(),
        InputKind::Entropy => s.entropy(),
        InputKind::InternalEnergy => s.internal_energy(),
        InputKind::Quality => s.quality()?,
    })
}
