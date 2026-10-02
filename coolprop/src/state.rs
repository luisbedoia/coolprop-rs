use crate::error::PropsError;
use crate::ffi::AbstractStateHandle;
use crate::input::Input;
use crate::phase::Phase;

/// Fully solved thermodynamic state; all properties computed eagerly in
/// `solve()`, so accessors are plain field reads.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct State {
    pressure: f64,
    temperature: f64,
    density: f64,
    enthalpy: f64,
    entropy: f64,
    internal_energy: f64,
    quality: Option<f64>,
    phase: Option<Phase>,
    cp: Option<f64>,
    cv: Option<f64>,
    viscosity: Option<f64>,
    conductivity: Option<f64>,
    prandtl: Option<f64>,
    gibbs: Option<f64>,
    compressibility: Option<f64>,
    speed_of_sound: Option<f64>,
}

impl State {
    pub(crate) fn solve(
        handle: &AbstractStateHandle,
        in1: Input,
        in2: Input,
    ) -> Result<Self, PropsError> {
        let v = handle.solve_state(in1, in2)?;
        Ok(Self {
            pressure: v.pressure,
            temperature: v.temperature,
            density: v.density,
            enthalpy: v.enthalpy,
            entropy: v.entropy,
            internal_energy: v.internal_energy,
            quality: v.quality,
            phase: v.phase,
            cp: v.cp,
            cv: v.cv,
            viscosity: v.viscosity,
            conductivity: v.conductivity,
            prandtl: v.prandtl,
            gibbs: v.gibbs,
            compressibility: v.compressibility,
            speed_of_sound: v.speed_of_sound,
        })
    }

    pub fn pressure(&self) -> f64 {
        self.pressure
    }

    pub fn temperature(&self) -> f64 {
        self.temperature
    }

    pub fn density(&self) -> f64 {
        self.density
    }

    pub fn enthalpy(&self) -> f64 {
        self.enthalpy
    }

    pub fn entropy(&self) -> f64 {
        self.entropy
    }

    pub fn internal_energy(&self) -> f64 {
        self.internal_energy
    }

    /// Mass quality `x ∈ [0, 1]`; `None` outside the two-phase region.
    pub fn quality(&self) -> Option<f64> {
        self.quality
    }

    /// `None` when CoolProp could not assign a phase.
    pub fn phase(&self) -> Option<Phase> {
        self.phase
    }

    /// Mass-based constant-pressure specific heat `cp`. `None` when undefined
    /// for the state (e.g. two-phase).
    pub fn cp(&self) -> Option<f64> {
        self.cp
    }

    /// Mass-based constant-volume specific heat `cv`. `None` when undefined.
    pub fn cv(&self) -> Option<f64> {
        self.cv
    }

    /// Dynamic viscosity. `None` when undefined (e.g. two-phase).
    pub fn viscosity(&self) -> Option<f64> {
        self.viscosity
    }

    /// Thermal conductivity. `None` when undefined (e.g. two-phase).
    pub fn conductivity(&self) -> Option<f64> {
        self.conductivity
    }

    /// Prandtl number. `None` when undefined (e.g. two-phase).
    pub fn prandtl(&self) -> Option<f64> {
        self.prandtl
    }

    /// Mass-based Gibbs free energy. `None` when undefined.
    pub fn gibbs(&self) -> Option<f64> {
        self.gibbs
    }

    /// Compressibility factor `Z = pv/RT`. `None` when undefined.
    pub fn compressibility(&self) -> Option<f64> {
        self.compressibility
    }

    /// Speed of sound. `None` when undefined (e.g. two-phase).
    pub fn speed_of_sound(&self) -> Option<f64> {
        self.speed_of_sound
    }
}
