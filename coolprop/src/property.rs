/// CoolProp parameter read from an `AbstractState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Property {
    Pressure,
    Temperature,
    Density,
    Enthalpy,
    Entropy,
    InternalEnergy,
    Quality,
    Phase,
    Cp,
    Cv,
    Viscosity,
    Conductivity,
    Prandtl,
    GibbsEnergy,
    Compressibility,
    SpeedOfSound,
}

impl Property {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Pressure => "P",
            Self::Temperature => "T",
            Self::Density => "D",
            Self::Enthalpy => "H",
            Self::Entropy => "S",
            Self::InternalEnergy => "U",
            Self::Quality => "Q",
            Self::Phase => "Phase",
            Self::Cp => "Cpmass",
            Self::Cv => "Cvmass",
            Self::Viscosity => "viscosity",
            Self::Conductivity => "conductivity",
            Self::Prandtl => "Prandtl",
            Self::GibbsEnergy => "Gmass",
            Self::Compressibility => "Z",
            Self::SpeedOfSound => "speed_of_sound",
        }
    }
}
