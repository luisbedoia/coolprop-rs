//! Self-description of the API: which inputs exist, which pairs of them fix
//! a state, which properties a [`State`](crate::State) reports and which
//! phases it can be in. Meant for building UIs from data instead of copying
//! these lists by hand.
//!
//! Units are SI; an empty `unit` means dimensionless. Descriptions and
//! labels come in the requested [`Locale`]; names, symbols and units do not
//! change with it. Unit conversion is up to the application.

use crate::input::{InputKind, PAIRS};
use crate::locale::{Locale, Text, text};
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
    /// Short name for display, e.g. "two-phase".
    pub label: &'static str,
    pub description: &'static str,
}

/// Every input, in [`InputKind::ALL`] order.
pub fn inputs(locale: Locale) -> &'static [InputInfo] {
    use InputKind::*;
    const fn input(
        locale: Locale,
        name: InputKind,
        symbol: &'static str,
        unit: &'static str,
        description: Text,
    ) -> InputInfo {
        InputInfo {
            name,
            symbol,
            unit,
            description: locale.pick(description),
            min: None,
            max: None,
        }
    }
    const fn table(l: Locale) -> [InputInfo; 7] {
        [
            input(l, Pressure, "p", "Pa", PRESSURE),
            input(l, Temperature, "T", "K", TEMPERATURE),
            input(l, Density, "ρ", "kg/m³", DENSITY),
            input(l, Enthalpy, "h", "J/kg", ENTHALPY),
            input(l, Entropy, "s", "J/(kg·K)", ENTROPY),
            input(l, InternalEnergy, "u", "J/kg", INTERNAL_ENERGY),
            InputInfo {
                min: Some(0.0),
                max: Some(1.0),
                ..input(
                    l,
                    Quality,
                    "x",
                    "",
                    text(
                        "Vapor quality (vapor mass fraction)",
                        "Calidad del vapor (fracción másica de vapor)",
                    ),
                )
            },
        ]
    }
    const INPUTS: [[InputInfo; 7]; Locale::ALL.len()] = [table(Locale::En), table(Locale::Es)];
    &INPUTS[locale as usize]
}

// Texts shared by inputs, state properties and plot properties.
const PRESSURE: Text = text("Pressure", "Presión");
const TEMPERATURE: Text = text("Temperature", "Temperatura");
const DENSITY: Text = text("Mass density", "Densidad");
const ENTHALPY: Text = text("Specific enthalpy", "Entalpía específica");
const ENTROPY: Text = text("Specific entropy", "Entropía específica");
const INTERNAL_ENERGY: Text = text("Specific internal energy", "Energía interna específica");

/// Every pair of inputs that fixes a state. Order within a pair does not
/// matter to [`Fluid::state`](crate::Fluid::state).
pub fn pairs() -> impl ExactSizeIterator<Item = (InputKind, InputKind)> {
    PAIRS.iter().map(|&(a, b, _)| (a, b))
}

/// Every numeric property of a [`State`](crate::State), in field order.
/// (`phase` is described by [`phases`].)
pub fn properties(locale: Locale) -> &'static [PropertyInfo] {
    use PropertyCategory::*;
    const fn prop(
        locale: Locale,
        name: &'static str,
        symbol: &'static str,
        unit: &'static str,
        description: Text,
        nullable: bool,
        category: PropertyCategory,
    ) -> PropertyInfo {
        PropertyInfo {
            name,
            symbol,
            unit,
            description: locale.pick(description),
            nullable,
            category,
        }
    }
    const fn table(l: Locale) -> [PropertyInfo; 15] {
        [
            prop(l, "pressure", "p", "Pa", PRESSURE, false, Thermodynamic),
            prop(
                l,
                "temperature",
                "T",
                "K",
                TEMPERATURE,
                false,
                Thermodynamic,
            ),
            prop(l, "density", "ρ", "kg/m³", DENSITY, false, Thermodynamic),
            prop(l, "enthalpy", "h", "J/kg", ENTHALPY, false, Thermodynamic),
            prop(l, "entropy", "s", "J/(kg·K)", ENTROPY, false, Thermodynamic),
            prop(
                l,
                "internal_energy",
                "u",
                "J/kg",
                INTERNAL_ENERGY,
                false,
                Thermodynamic,
            ),
            prop(
                l,
                "quality",
                "x",
                "",
                text(
                    "Vapor quality; only inside the two-phase region",
                    "Calidad del vapor; solo dentro de la región bifásica",
                ),
                true,
                Thermodynamic,
            ),
            prop(
                l,
                "cp",
                "cp",
                "J/(kg·K)",
                text(
                    "Specific heat at constant pressure",
                    "Calor específico a presión constante",
                ),
                true,
                Thermodynamic,
            ),
            prop(
                l,
                "cv",
                "cv",
                "J/(kg·K)",
                text(
                    "Specific heat at constant volume",
                    "Calor específico a volumen constante",
                ),
                true,
                Thermodynamic,
            ),
            prop(
                l,
                "viscosity",
                "μ",
                "Pa·s",
                text("Dynamic viscosity", "Viscosidad dinámica"),
                true,
                Transport,
            ),
            prop(
                l,
                "conductivity",
                "λ",
                "W/(m·K)",
                text("Thermal conductivity", "Conductividad térmica"),
                true,
                Transport,
            ),
            prop(
                l,
                "prandtl",
                "Pr",
                "",
                text("Prandtl number", "Número de Prandtl"),
                true,
                Transport,
            ),
            prop(
                l,
                "gibbs",
                "g",
                "J/kg",
                text("Specific Gibbs energy", "Energía de Gibbs específica"),
                true,
                Thermodynamic,
            ),
            prop(
                l,
                "compressibility",
                "Z",
                "",
                text(
                    "Compressibility factor pv/(RT)",
                    "Factor de compresibilidad pv/(RT)",
                ),
                true,
                Thermodynamic,
            ),
            prop(
                l,
                "speed_of_sound",
                "a",
                "m/s",
                text("Speed of sound", "Velocidad del sonido"),
                true,
                Thermodynamic,
            ),
        ]
    }
    const PROPERTIES: [[PropertyInfo; 15]; Locale::ALL.len()] =
        [table(Locale::En), table(Locale::Es)];
    &PROPERTIES[locale as usize]
}

/// Every phase, in [`Phase::ALL`] order.
pub fn phases(locale: Locale) -> impl ExactSizeIterator<Item = PhaseInfo> {
    Phase::ALL.into_iter().map(move |name| PhaseInfo {
        name,
        label: name.label(locale),
        description: name.description(locale),
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
pub fn plot_properties(locale: Locale) -> &'static [PlotPropertyInfo] {
    use PlotProperty::*;
    const fn info(
        locale: Locale,
        name: PlotProperty,
        symbol: &'static str,
        unit: &'static str,
        description: Text,
    ) -> PlotPropertyInfo {
        PlotPropertyInfo {
            name,
            symbol,
            unit,
            description: locale.pick(description),
        }
    }
    const fn table(l: Locale) -> [PlotPropertyInfo; 7] {
        [
            info(l, Pressure, "p", "Pa", PRESSURE),
            info(l, Temperature, "T", "K", TEMPERATURE),
            info(l, Density, "ρ", "kg/m³", DENSITY),
            info(
                l,
                SpecificVolume,
                "v",
                "m³/kg",
                text("Specific volume", "Volumen específico"),
            ),
            info(l, Enthalpy, "h", "J/kg", ENTHALPY),
            info(l, Entropy, "s", "J/(kg·K)", ENTROPY),
            info(l, InternalEnergy, "u", "J/kg", INTERNAL_ENERGY),
        ]
    }
    const PLOT_PROPERTIES: [[PlotPropertyInfo; 7]; Locale::ALL.len()] =
        [table(Locale::En), table(Locale::Es)];
    &PLOT_PROPERTIES[locale as usize]
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
