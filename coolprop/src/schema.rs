//! Self-description of the API: which inputs exist, which pairs of them fix
//! a state, which properties a [`State`](crate::State) reports and which
//! phases it can be in. Meant for building UIs from data instead of copying
//! these lists by hand.
//!
//! Units are SI; an empty `unit` means dimensionless. Texts are English;
//! labels and unit conversion are up to the application.

use crate::input::{InputKind, PAIRS};
use crate::phase::Phase;
use crate::plot::{Axis, Diagram, PlotProperty};

/// An input that can fix a state.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct InputInfo {
    /// API name, e.g. `"pressure"`.
    pub name: InputKind,
    /// Conventional symbol, e.g. `"p"`.
    pub symbol: &'static str,
    pub unit: &'static str,
    pub description: &'static str,
    /// Inclusive bounds when the input has fixed ones (only `quality`).
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Groups state properties for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PropertyCategory {
    Thermodynamic,
    Transport,
}

/// A numeric property of a solved [`State`](crate::State).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PropertyInfo {
    /// Field name in the serialized `State`, e.g. `"speed_of_sound"`.
    pub name: &'static str,
    pub symbol: &'static str,
    pub unit: &'static str,
    pub description: &'static str,
    /// Whether the value can be missing for some states (e.g. `quality`
    /// outside the two-phase region, `cp` inside it).
    pub nullable: bool,
    pub category: PropertyCategory,
}

/// A phase a [`State`](crate::State) can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PhaseInfo {
    pub name: Phase,
    pub description: &'static str,
}

/// Every input, in [`InputKind::ALL`] order.
pub fn inputs() -> &'static [InputInfo] {
    use InputKind::*;
    const fn input(
        name: InputKind,
        symbol: &'static str,
        unit: &'static str,
        description: &'static str,
    ) -> InputInfo {
        InputInfo {
            name,
            symbol,
            unit,
            description,
            min: None,
            max: None,
        }
    }
    const INPUTS: [InputInfo; 7] = [
        input(Pressure, "p", "Pa", "Pressure"),
        input(Temperature, "T", "K", "Temperature"),
        input(Density, "ρ", "kg/m³", "Mass density"),
        input(Enthalpy, "h", "J/kg", "Specific enthalpy"),
        input(Entropy, "s", "J/(kg·K)", "Specific entropy"),
        input(InternalEnergy, "u", "J/kg", "Specific internal energy"),
        InputInfo {
            min: Some(0.0),
            max: Some(1.0),
            ..input(Quality, "x", "", "Vapor quality (vapor mass fraction)")
        },
    ];
    &INPUTS
}

/// Every pair of inputs that fixes a state. Order within a pair does not
/// matter to [`Fluid::state`](crate::Fluid::state).
pub fn pairs() -> impl ExactSizeIterator<Item = (InputKind, InputKind)> {
    PAIRS.iter().map(|&(a, b, _)| (a, b))
}

/// Every numeric property of a [`State`](crate::State), in field order.
/// (`phase` is described by [`phases`].)
pub fn properties() -> &'static [PropertyInfo] {
    use PropertyCategory::*;
    const fn prop(
        name: &'static str,
        symbol: &'static str,
        unit: &'static str,
        description: &'static str,
        nullable: bool,
        category: PropertyCategory,
    ) -> PropertyInfo {
        PropertyInfo {
            name,
            symbol,
            unit,
            description,
            nullable,
            category,
        }
    }
    const PROPERTIES: [PropertyInfo; 15] = [
        prop("pressure", "p", "Pa", "Pressure", false, Thermodynamic),
        prop("temperature", "T", "K", "Temperature", false, Thermodynamic),
        prop(
            "density",
            "ρ",
            "kg/m³",
            "Mass density",
            false,
            Thermodynamic,
        ),
        prop(
            "enthalpy",
            "h",
            "J/kg",
            "Specific enthalpy",
            false,
            Thermodynamic,
        ),
        prop(
            "entropy",
            "s",
            "J/(kg·K)",
            "Specific entropy",
            false,
            Thermodynamic,
        ),
        prop(
            "internal_energy",
            "u",
            "J/kg",
            "Specific internal energy",
            false,
            Thermodynamic,
        ),
        prop(
            "quality",
            "x",
            "",
            "Vapor quality; only inside the two-phase region",
            true,
            Thermodynamic,
        ),
        prop(
            "cp",
            "cp",
            "J/(kg·K)",
            "Specific heat at constant pressure",
            true,
            Thermodynamic,
        ),
        prop(
            "cv",
            "cv",
            "J/(kg·K)",
            "Specific heat at constant volume",
            true,
            Thermodynamic,
        ),
        prop(
            "viscosity",
            "μ",
            "Pa·s",
            "Dynamic viscosity",
            true,
            Transport,
        ),
        prop(
            "conductivity",
            "λ",
            "W/(m·K)",
            "Thermal conductivity",
            true,
            Transport,
        ),
        prop("prandtl", "Pr", "", "Prandtl number", true, Transport),
        prop(
            "gibbs",
            "g",
            "J/kg",
            "Specific Gibbs energy",
            true,
            Thermodynamic,
        ),
        prop(
            "compressibility",
            "Z",
            "",
            "Compressibility factor pv/(RT)",
            true,
            Thermodynamic,
        ),
        prop(
            "speed_of_sound",
            "a",
            "m/s",
            "Speed of sound",
            true,
            Thermodynamic,
        ),
    ];
    &PROPERTIES
}

/// Every phase, in [`Phase::ALL`] order.
pub fn phases() -> impl ExactSizeIterator<Item = PhaseInfo> {
    Phase::ALL.into_iter().map(|name| PhaseInfo {
        name,
        description: name.description(),
    })
}

/// A property that can be plotted on a diagram axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PlotPropertyInfo {
    pub name: PlotProperty,
    pub symbol: &'static str,
    pub unit: &'static str,
    pub description: &'static str,
}

/// Every plottable property, in [`PlotProperty::ALL`] order.
pub fn plot_properties() -> &'static [PlotPropertyInfo] {
    use PlotProperty::*;
    const fn info(
        name: PlotProperty,
        symbol: &'static str,
        unit: &'static str,
        description: &'static str,
    ) -> PlotPropertyInfo {
        PlotPropertyInfo {
            name,
            symbol,
            unit,
            description,
        }
    }
    const PLOT_PROPERTIES: [PlotPropertyInfo; 7] = [
        info(Pressure, "p", "Pa", "Pressure"),
        info(Temperature, "T", "K", "Temperature"),
        info(Density, "ρ", "kg/m³", "Mass density"),
        info(SpecificVolume, "v", "m³/kg", "Specific volume"),
        info(Enthalpy, "h", "J/kg", "Specific enthalpy"),
        info(Entropy, "s", "J/(kg·K)", "Specific entropy"),
        info(InternalEnergy, "u", "J/kg", "Specific internal energy"),
    ];
    &PLOT_PROPERTIES
}

/// A diagram of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DiagramInfo {
    /// `"<y>_<x>"`, e.g. `"pressure_enthalpy"`.
    pub id: String,
    pub x: Axis,
    pub y: Axis,
    /// Isoline families worth drawing on it.
    pub isolines: Vec<InputKind>,
}

/// Every diagram of [`Diagram::all`].
pub fn diagrams() -> Vec<DiagramInfo> {
    Diagram::all()
        .into_iter()
        .map(|d| DiagramInfo {
            id: d.id(),
            x: d.x,
            y: d.y,
            isolines: d.isoline_kinds(),
        })
        .collect()
}
