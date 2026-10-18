/// Where a fluid's enthalpy and entropy are measured from. Only their
/// absolute values change: differences (and every other property) do not.
///
/// The textbooks' refrigerant tables (Çengel, Moran) use [`Ashrae`]; most
/// of CoolProp's refrigerants default to [`Iir`]. Water's default is
/// already the steam tables' (u = s = 0 for the liquid at the triple point).
///
/// [`Ashrae`]: Reference::Ashrae
/// [`Iir`]: Reference::Iir
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Reference {
    /// The fluid's own, from its CoolProp equation of state.
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "DEF"))]
    Default,
    /// h = 200 kJ/kg, s = 1 kJ/(kg·K) for the saturated liquid at 0 °C.
    /// For fluids whose triple point is below 0 °C.
    #[cfg_attr(feature = "serde", serde(rename = "IIR"))]
    Iir,
    /// h = s = 0 for the saturated liquid at −40 °C. For fluids whose
    /// triple point is below −40 °C.
    #[cfg_attr(feature = "serde", serde(rename = "ASHRAE"))]
    Ashrae,
    /// h = s = 0 for the saturated liquid at 1 atm (the normal boiling
    /// point). For fluids whose triple point is below 1 atm.
    #[cfg_attr(feature = "serde", serde(rename = "NBP"))]
    Nbp,
}

impl Reference {
    pub const ALL: [Reference; 4] = [
        Reference::Default,
        Reference::Iir,
        Reference::Ashrae,
        Reference::Nbp,
    ];

    /// CoolProp's name, also the serde JSON one: `"DEF"`, `"IIR"`, ….
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "DEF",
            Self::Iir => "IIR",
            Self::Ashrae => "ASHRAE",
            Self::Nbp => "NBP",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.name() == name)
    }
}
