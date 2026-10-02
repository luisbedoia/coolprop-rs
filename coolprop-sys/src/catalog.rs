/// Curated-catalog constants for one fluid, in SI units, read from
/// `CoolProp/dev/fluids/*.json` (the parenthetical is the source field).
///
/// Only values CoolProp uses verbatim are listed, so each one matches the
/// runtime EOS exactly. The critical point is not: CoolProp recomputes it
/// (up to ~4 % off the published density), so it comes from the EOS at
/// runtime instead.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FluidData {
    /// Canonical CoolProp name (`INFO.NAME`).
    pub name: &'static str,
    /// CAS registry number (`INFO.CAS`).
    pub cas: &'static str,
    /// Formula in CoolProp's LaTeX-ish notation (`INFO.FORMULA`); empty for
    /// pseudo-pure fluids such as Air.
    pub formula: &'static str,
    /// Other names CoolProp accepts, e.g. `"H2O"`, `"R718"` (`INFO.ALIASES`).
    pub aliases: &'static [&'static str],
    /// kg/mol (`EOS[0].molar_mass`).
    pub molar_mass: f64,
    /// Acentric factor, dimensionless (`EOS[0].acentric`).
    pub acentric: f64,
    /// K — triple point, also the EOS lower temperature limit
    /// (`EOS[0].STATES.sat_min_liquid.T`).
    pub t_triple: f64,
    /// Pa — triple point (`EOS[0].STATES.sat_min_liquid.p`).
    pub p_triple: f64,
    /// K — EOS upper temperature limit (`EOS[0].T_max`).
    pub t_max: f64,
    /// Pa — EOS upper pressure limit (`EOS[0].p_max`).
    pub p_max: f64,
}

include!(concat!(env!("OUT_DIR"), "/catalog_gen.rs"));

impl Variant {
    /// Looks up a curated fluid by its canonical name or any CoolProp alias
    /// (exact match; the aliases already cover common spellings such as
    /// `"water"`/`"WATER"`). `None` for any non-curated name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|v| v.name() == name || v.data().aliases.contains(&name))
    }
}
