use crate::property::Property;

/// One of the two independent properties that fix a state in
/// [`crate::Fluid::state`]. Values are SI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    /// Pa
    Pressure(f64),
    /// K
    Temperature(f64),
    /// kg/m³
    Density(f64),
    /// J/kg
    Enthalpy(f64),
    /// J/(kg·K)
    Entropy(f64),
    /// J/kg
    InternalEnergy(f64),
    /// Vapor mass fraction, dimensionless, in [0, 1].
    Quality(f64),
}

impl Input {
    pub(crate) const fn property(self) -> Property {
        match self {
            Self::Pressure(_) => Property::Pressure,
            Self::Temperature(_) => Property::Temperature,
            Self::Density(_) => Property::Density,
            Self::Enthalpy(_) => Property::Enthalpy,
            Self::Entropy(_) => Property::Entropy,
            Self::InternalEnergy(_) => Property::InternalEnergy,
            Self::Quality(_) => Property::Quality,
        }
    }

    pub(crate) const fn value(self) -> f64 {
        match self {
            Self::Pressure(v)
            | Self::Temperature(v)
            | Self::Density(v)
            | Self::Enthalpy(v)
            | Self::Entropy(v)
            | Self::InternalEnergy(v)
            | Self::Quality(v) => v,
        }
    }
}
