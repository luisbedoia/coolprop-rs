//! Display names of every fluid CoolProp ships (`CoolProp/dev/fluids`), so
//! any catalog built with `COOLPROP_FLUIDS` has them. Refrigerant numbers
//! and trade names are the same in every locale.

use crate::locale::{Text, text};

/// The display name of the fluid whose canonical CoolProp name is `name`.
pub(crate) fn display_name(name: &str) -> Option<Text> {
    let same = |s| Some(text(s, s));
    if let Some(&n) = AS_IS.iter().find(|&&n| n == name) {
        return same(n);
    }
    match name {
        "1-Butene" => Some(text("1-Butene", "1-Buteno")),
        "Acetone" => Some(text("Acetone", "Acetona")),
        "Air" => Some(text("Air", "Aire")),
        "Ammonia" => Some(text("Ammonia", "Amoníaco")),
        "Argon" => Some(text("Argon", "Argón")),
        "Benzene" => Some(text("Benzene", "Benceno")),
        "CarbonDioxide" => Some(text("Carbon dioxide", "Dióxido de carbono")),
        "CarbonMonoxide" => Some(text("Carbon monoxide", "Monóxido de carbono")),
        "CarbonylSulfide" => Some(text("Carbonyl sulfide", "Sulfuro de carbonilo")),
        "Chlorine" => Some(text("Chlorine", "Cloro")),
        "CycloHexane" => Some(text("Cyclohexane", "Ciclohexano")),
        "CycloPropane" => Some(text("Cyclopropane", "Ciclopropano")),
        "Cyclopentane" => Some(text("Cyclopentane", "Ciclopentano")),
        "D4" => Some(text(
            "Octamethylcyclotetrasiloxane (D4)",
            "Octametilciclotetrasiloxano (D4)",
        )),
        "D5" => Some(text(
            "Decamethylcyclopentasiloxane (D5)",
            "Decametilciclopentasiloxano (D5)",
        )),
        "D6" => Some(text(
            "Dodecamethylcyclohexasiloxane (D6)",
            "Dodecametilciclohexasiloxano (D6)",
        )),
        "Deuterium" => Some(text("Deuterium", "Deuterio")),
        "Dichloroethane" => Some(text("1,2-Dichloroethane", "1,2-Dicloroetano")),
        "DiethylEther" => Some(text("Diethyl ether", "Éter dietílico")),
        "DimethylCarbonate" => Some(text("Dimethyl carbonate", "Carbonato de dimetilo")),
        "DimethylEther" => Some(text("Dimethyl ether", "Éter dimetílico")),
        "Ethane" => Some(text("Ethane", "Etano")),
        "Ethanol" => Some(text("Ethanol", "Etanol")),
        "EthylBenzene" => Some(text("Ethylbenzene", "Etilbenceno")),
        "Ethylene" => Some(text("Ethylene", "Etileno")),
        "EthyleneOxide" => Some(text("Ethylene oxide", "Óxido de etileno")),
        "Fluorine" => Some(text("Fluorine", "Flúor")),
        "HFE143m" => same("HFE-143m"),
        "HeavyWater" => Some(text("Heavy water", "Agua pesada")),
        "Helium" => Some(text("Helium", "Helio")),
        "Hydrogen" => Some(text("Hydrogen", "Hidrógeno")),
        "HydrogenChloride" => Some(text("Hydrogen chloride", "Cloruro de hidrógeno")),
        "HydrogenSulfide" => Some(text("Hydrogen sulfide", "Sulfuro de hidrógeno")),
        "IsoButane" => Some(text("Isobutane", "Isobutano")),
        "IsoButene" => Some(text("Isobutene", "Isobuteno")),
        "Isohexane" => Some(text("Isohexane", "Isohexano")),
        "Isopentane" => Some(text("Isopentane", "Isopentano")),
        "Krypton" => Some(text("Krypton", "Kriptón")),
        "MD2M" => Some(text(
            "Decamethyltetrasiloxane (MD2M)",
            "Decametiltetrasiloxano (MD2M)",
        )),
        "MD3M" => Some(text(
            "Dodecamethylpentasiloxane (MD3M)",
            "Dodecametilpentasiloxano (MD3M)",
        )),
        "MD4M" => Some(text(
            "Tetradecamethylhexasiloxane (MD4M)",
            "Tetradecametilhexasiloxano (MD4M)",
        )),
        "MDM" => Some(text(
            "Octamethyltrisiloxane (MDM)",
            "Octametiltrisiloxano (MDM)",
        )),
        "MM" => Some(text(
            "Hexamethyldisiloxane (MM)",
            "Hexametildisiloxano (MM)",
        )),
        "Methane" => Some(text("Methane", "Metano")),
        "Methanol" => Some(text("Methanol", "Metanol")),
        "MethylLinoleate" => Some(text("Methyl linoleate", "Linoleato de metilo")),
        "MethylLinolenate" => Some(text("Methyl linolenate", "Linolenato de metilo")),
        "MethylOleate" => Some(text("Methyl oleate", "Oleato de metilo")),
        "MethylPalmitate" => Some(text("Methyl palmitate", "Palmitato de metilo")),
        "MethylStearate" => Some(text("Methyl stearate", "Estearato de metilo")),
        "Neon" => Some(text("Neon", "Neón")),
        "Neopentane" => Some(text("Neopentane", "Neopentano")),
        "Nitrogen" => Some(text("Nitrogen", "Nitrógeno")),
        "NitrousOxide" => Some(text("Nitrous oxide", "Óxido nitroso")),
        "Novec649" => same("Novec 649"),
        "OrthoDeuterium" => Some(text("Orthodeuterium", "Ortodeuterio")),
        "OrthoHydrogen" => Some(text("Orthohydrogen", "Ortohidrógeno")),
        "Oxygen" => Some(text("Oxygen", "Oxígeno")),
        "ParaDeuterium" => Some(text("Paradeuterium", "Paradeuterio")),
        "ParaHydrogen" => Some(text("Parahydrogen", "Parahidrógeno")),
        "Propylene" => Some(text("Propylene", "Propileno")),
        "PropyleneGlycol" => Some(text("Propylene glycol", "Propilenglicol")),
        "Propyne" => Some(text("Propyne", "Propino")),
        "SulfurDioxide" => Some(text("Sulfur dioxide", "Dióxido de azufre")),
        "SulfurHexafluoride" => Some(text("Sulfur hexafluoride", "Hexafluoruro de azufre")),
        "Tetrahydrofuran" => Some(text("Tetrahydrofuran", "Tetrahidrofurano")),
        "Toluene" => Some(text("Toluene", "Tolueno")),
        "VinylChloride" => Some(text("Vinyl chloride", "Cloruro de vinilo")),
        "Water" => Some(text("Water", "Agua")),
        "Xenon" => Some(text("Xenon", "Xenón")),
        "cis-2-Butene" => Some(text("cis-2-Butene", "cis-2-Buteno")),
        "m-Xylene" => Some(text("m-Xylene", "m-Xileno")),
        "n-Butane" => Some(text("n-Butane", "n-Butano")),
        "n-Decane" => Some(text("n-Decane", "n-Decano")),
        "n-Dodecane" => Some(text("n-Dodecane", "n-Dodecano")),
        "n-Heptane" => Some(text("n-Heptane", "n-Heptano")),
        "n-Hexane" => Some(text("n-Hexane", "n-Hexano")),
        "n-Nonane" => Some(text("n-Nonane", "n-Nonano")),
        "n-Octane" => Some(text("n-Octane", "n-Octano")),
        "n-Pentane" => Some(text("n-Pentane", "n-Pentano")),
        "n-Perfluorobutane" => Some(text("n-Perfluorobutane", "n-Perfluorobutano")),
        "n-Perfluorohexane" => Some(text("n-Perfluorohexane", "n-Perfluorohexano")),
        "n-Perfluoropentane" => Some(text("n-Perfluoropentane", "n-Perfluoropentano")),
        "n-Propane" => Some(text("Propane", "Propano")),
        "n-Undecane" => Some(text("n-Undecane", "n-Undecano")),
        "o-Xylene" => Some(text("o-Xylene", "o-Xileno")),
        "p-Xylene" => Some(text("p-Xylene", "p-Xileno")),
        "trans-2-Butene" => Some(text("trans-2-Butene", "trans-2-Buteno")),
        // Refrigerant numbers in ASHRAE 34 case, where CoolProp's differs.
        "R152A" => same("R152a"),
        "R1224YDZ" => same("R1224yd(Z)"),
        "R227EA" => same("R227ea"),
        "R236EA" => same("R236ea"),
        "R236FA" => same("R236fa"),
        "R365MFC" => same("R365mfc"),
        _ => None,
    }
}

/// Fluids shown by their canonical name: refrigerant numbers and trade names.
const AS_IS: &[&str] = &[
    "R11",
    "R1123",
    "R113",
    "R1130(E)",
    "R1132(E)",
    "R114",
    "R115",
    "R116",
    "R12",
    "R123",
    "R1233zd(E)",
    "R1234yf",
    "R1234ze(E)",
    "R1234ze(Z)",
    "R124",
    "R1243zf",
    "R125",
    "R13",
    "R1336mzz(E)",
    "R1336mzz(Z)",
    "R134a",
    "R13I1",
    "R14",
    "R141b",
    "R142b",
    "R143a",
    "R161",
    "R21",
    "R218",
    "R22",
    "R23",
    "R245ca",
    "R245fa",
    "R32",
    "R40",
    "R404A",
    "R407C",
    "R41",
    "R410A",
    "R507A",
    "RC318",
    "SES36",
];

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::Locale;

    /// Every fluid CoolProp ships has a name in every locale, so no catalog
    /// built with `COOLPROP_FLUIDS` falls back to the canonical name.
    #[test]
    fn every_coolprop_fluid_has_a_display_name() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../coolprop-sys/CoolProp/dev/fluids");
        let mut missing = Vec::new();
        let mut count = 0;
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let json: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            let name = json["INFO"]["NAME"].as_str().unwrap().to_owned();
            count += 1;
            match display_name(&name) {
                Some(t) if Locale::ALL.iter().all(|l| !l.pick(t).is_empty()) => {}
                _ => missing.push(name),
            }
        }
        assert!(
            count > 100,
            "read only {count} fluids from {}",
            dir.display()
        );
        assert!(missing.is_empty(), "no display name for {missing:?}");
    }
}
