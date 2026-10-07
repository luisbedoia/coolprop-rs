//! Read and parse curated CoolProp fluid JSONs.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Default catalog: file stems of `CoolProp/dev/fluids/{stem}.json`. The
/// runtime name comes from each JSON's `INFO.NAME` (may differ from the stem).
/// Overridable via the `COOLPROP_FLUIDS` env var. A missing JSON or field
/// fails the build.
pub(crate) const CURATED_FLUIDS: &[&str] = &[
    "Air",
    "Water",
    "Ammonia",
    "CarbonDioxide",
    "Nitrogen",
    "Oxygen",
    "Methane",
    "Ethane",
    "R134a",
    "R245fa",
    "R1234yf",
    "R1233zd(E)",
    "n-Propane",
    "IsoButane",
    "n-Pentane",
    "Isopentane",
];

/// `COOLPROP_FLUIDS` (comma-separated, trimmed) when non-empty, else
/// [`CURATED_FLUIDS`].
pub(crate) fn resolve_fluids() -> Vec<String> {
    match env::var("COOLPROP_FLUIDS") {
        Ok(raw) if !raw.trim().is_empty() => raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .fold(Vec::new(), |mut acc, stem| {
                if !acc.contains(&stem) {
                    acc.push(stem);
                }
                acc
            }),
        _ => CURATED_FLUIDS.iter().map(|s| s.to_string()).collect(),
    }
}

pub(crate) fn fluids_dir(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join("CoolProp/dev/fluids")
}

pub(crate) struct FluidFields {
    pub(crate) name: String,
    pub(crate) cas: String,
    pub(crate) formula: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) molar_mass: f64,
    pub(crate) acentric: f64,
    pub(crate) t_triple: f64,
    pub(crate) p_triple: f64,
    pub(crate) t_max: f64,
    pub(crate) p_max: f64,
    pub(crate) pseudo_pure: bool,
}

pub(crate) fn load_fluid_json(fluids_dir: &Path, file_stem: &str) -> serde_json::Value {
    let path = fluids_dir.join(format!("{file_stem}.json"));
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "fluid '{file_stem}' declared in CURATED_FLUIDS but {} cannot be read: {e}",
            path.display()
        )
    });
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("fluid '{file_stem}': invalid JSON: {e}"))
}

pub(crate) fn read_fluid(fluids_dir: &Path, file_stem: &str) -> FluidFields {
    let json = load_fluid_json(fluids_dir, file_stem);

    let info = json
        .get("INFO")
        .unwrap_or_else(|| panic!("fluid '{file_stem}': missing INFO"));
    let ctx_info = format!("fluid '{file_stem}' (INFO)");
    let name = extract_str(info, "NAME", &ctx_info);
    let cas = extract_str(info, "CAS", &ctx_info);
    // Pseudo-pure fluids (e.g. Air) have `FORMULA: null`; treat as empty.
    let formula = info
        .get("FORMULA")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let aliases = info
        .get("ALIASES")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str())
                .filter(|a| *a != name)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let eos0 = json
        .get("EOS")
        .and_then(|e| e.get(0))
        .unwrap_or_else(|| panic!("fluid '{file_stem}': missing EOS[0]"));
    let ctx_eos = format!("fluid '{file_stem}' (EOS[0])");
    let molar_mass = extract_f64(eos0, "molar_mass", &ctx_eos);
    let acentric = extract_f64(eos0, "acentric", &ctx_eos);
    let t_max = extract_f64(eos0, "T_max", &ctx_eos);
    let p_max = extract_f64(eos0, "p_max", &ctx_eos);
    let pseudo_pure = eos0
        .get("pseudo_pure")
        .and_then(|x| x.as_bool())
        .unwrap_or_else(|| panic!("{ctx_eos}: missing or non-boolean field `pseudo_pure`"));

    // CoolProp takes both its lower temperature limit (Tmin) and the triple
    // point from here, not from `EOS[0].Ttriple` (see FluidLibrary.h).
    let sat_min = eos0
        .get("STATES")
        .and_then(|s| s.get("sat_min_liquid"))
        .unwrap_or_else(|| panic!("fluid '{file_stem}': missing EOS[0].STATES.sat_min_liquid"));
    let ctx_sat = format!("fluid '{file_stem}' (EOS[0].STATES.sat_min_liquid)");
    let t_triple = extract_f64(sat_min, "T", &ctx_sat);
    let p_triple = extract_f64(sat_min, "p", &ctx_sat);

    FluidFields {
        name,
        cas,
        formula,
        aliases,
        molar_mass,
        acentric,
        t_triple,
        p_triple,
        t_max,
        p_max,
        pseudo_pure,
    }
}

/// CAS numbers of every curated fluid; filter key for slimming the mixture
/// BIP table.
pub(crate) fn curated_cas_set(manifest_dir: &Path, fluids: &[String]) -> HashSet<String> {
    let dir = fluids_dir(manifest_dir);
    fluids
        .iter()
        .map(|stem| read_fluid(&dir, stem).cas)
        .collect()
}

fn extract_str(v: &serde_json::Value, key: &str, ctx: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_else(|| panic!("{ctx}: missing or non-string field `{key}`"))
        .to_string()
}

fn extract_f64(v: &serde_json::Value, key: &str, ctx: &str) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64())
        .unwrap_or_else(|| panic!("{ctx}: missing or non-numeric field `{key}`"))
}
