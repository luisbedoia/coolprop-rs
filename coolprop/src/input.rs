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

/// Which property an [`Input`] sets, without its value. Serialized (with the
/// `serde` feature) as the snake_case name used across the API, e.g.
/// `"internal_energy"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum InputKind {
    Pressure,
    Temperature,
    Density,
    Enthalpy,
    Entropy,
    InternalEnergy,
    Quality,
}

/// Every input pair CoolProp can solve, in the order of CoolProp's own
/// signature, with its `*_INPUTS` name. Single source for both
/// `Fluid::state` and [`crate::schema::pairs`].
pub(crate) const PAIRS: &[(InputKind, InputKind, &str)] = {
    use InputKind::*;
    &[
        (Pressure, Temperature, "PT_INPUTS"),
        (Pressure, Quality, "PQ_INPUTS"),
        (Quality, Temperature, "QT_INPUTS"),
        (Density, Pressure, "DmassP_INPUTS"),
        (Enthalpy, Pressure, "HmassP_INPUTS"),
        (Pressure, Entropy, "PSmass_INPUTS"),
        (Pressure, InternalEnergy, "PUmass_INPUTS"),
        (Density, Temperature, "DmassT_INPUTS"),
        (Entropy, Temperature, "SmassT_INPUTS"),
        (Density, Quality, "DmassQ_INPUTS"),
        (Density, Enthalpy, "DmassHmass_INPUTS"),
        (Density, Entropy, "DmassSmass_INPUTS"),
        (Density, InternalEnergy, "DmassUmass_INPUTS"),
        (Enthalpy, Entropy, "HmassSmass_INPUTS"),
    ]
};

impl InputKind {
    pub const ALL: [InputKind; 7] = [
        Self::Pressure,
        Self::Temperature,
        Self::Density,
        Self::Enthalpy,
        Self::Entropy,
        Self::InternalEnergy,
        Self::Quality,
    ];

    /// The snake_case name used across the API (`"pressure"`, …).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pressure => "pressure",
            Self::Temperature => "temperature",
            Self::Density => "density",
            Self::Enthalpy => "enthalpy",
            Self::Entropy => "entropy",
            Self::InternalEnergy => "internal_energy",
            Self::Quality => "quality",
        }
    }

    /// Inverse of [`Self::name`].
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }

    /// This input with a value.
    pub const fn with_value(self, value: f64) -> Input {
        match self {
            Self::Pressure => Input::Pressure(value),
            Self::Temperature => Input::Temperature(value),
            Self::Density => Input::Density(value),
            Self::Enthalpy => Input::Enthalpy(value),
            Self::Entropy => Input::Entropy(value),
            Self::InternalEnergy => Input::InternalEnergy(value),
            Self::Quality => Input::Quality(value),
        }
    }
}

impl Input {
    pub const fn kind(self) -> InputKind {
        match self {
            Self::Pressure(_) => InputKind::Pressure,
            Self::Temperature(_) => InputKind::Temperature,
            Self::Density(_) => InputKind::Density,
            Self::Enthalpy(_) => InputKind::Enthalpy,
            Self::Entropy(_) => InputKind::Entropy,
            Self::InternalEnergy(_) => InputKind::InternalEnergy,
            Self::Quality(_) => InputKind::Quality,
        }
    }

    pub const fn value(self) -> f64 {
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
