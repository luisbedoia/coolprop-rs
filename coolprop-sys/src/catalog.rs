/// Curated-catalog constants for one fluid, in SI units. The parenthetical
/// is the source field in `CoolProp/dev/fluids/*.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FluidData {
    /// Canonical CoolProp name accepted by `AbstractState_factory` (`INFO.NAME`).
    pub name: &'static str,
    /// CAS registry number (`INFO.CAS`).
    pub cas: &'static str,
    /// Formula in CoolProp's LaTeX-ish notation (`INFO.FORMULA`).
    pub formula: &'static str,
    /// kg/mol (`EOS[0].molar_mass`).
    pub molar_mass: f64,
    /// K (`STATES.critical.T`).
    pub t_critical: f64,
    /// Pa (`STATES.critical.p`).
    pub p_critical: f64,
    /// K — EOS lower bound (`EOS[0].Ttriple`).
    pub t_min: f64,
    /// K — EOS upper bound (`EOS[0].T_max`).
    pub t_max: f64,
    /// Pa — EOS upper bound (`EOS[0].p_max`).
    pub p_max: f64,
}

include!(concat!(env!("OUT_DIR"), "/catalog_gen.rs"));

impl Variant {
    /// Matches a canonical CoolProp name to a `Variant`. No alias handling;
    /// `None` for any non-curated name.
    pub fn from_name(name: &str) -> Option<Self> {
        let normalized = normalize_name(name);
        Self::ALL
            .iter()
            .copied()
            .find(|v| normalize_name(v.name()) == normalized)
    }
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}
