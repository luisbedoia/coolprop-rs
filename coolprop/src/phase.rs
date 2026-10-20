use std::fmt;

use crate::locale::{Locale, text};

/// Phase classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Phase {
    Liquid,
    Supercritical,
    SupercriticalGas,
    SupercriticalLiquid,
    CriticalPoint,
    Gas,
    TwoPhase,
}

impl Phase {
    pub const ALL: [Phase; 7] = [
        Self::Liquid,
        Self::Supercritical,
        Self::SupercriticalGas,
        Self::SupercriticalLiquid,
        Self::CriticalPoint,
        Self::Gas,
        Self::TwoPhase,
    ];

    /// `None` for CoolProp's `NotImposed`/`Unknown` sentinels or any
    /// non-finite/out-of-range index.
    pub(crate) fn from_index(index: f64) -> Option<Self> {
        if !index.is_finite() {
            return None;
        }
        match index.round() as i64 {
            0 => Some(Self::Liquid),
            1 => Some(Self::Supercritical),
            2 => Some(Self::SupercriticalGas),
            3 => Some(Self::SupercriticalLiquid),
            4 => Some(Self::CriticalPoint),
            5 => Some(Self::Gas),
            6 => Some(Self::TwoPhase),
            _ => None,
        }
    }

    /// snake_case name, matching the serde JSON representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Liquid => "liquid",
            Self::Supercritical => "supercritical",
            Self::SupercriticalGas => "supercritical_gas",
            Self::SupercriticalLiquid => "supercritical_liquid",
            Self::CriticalPoint => "critical_point",
            Self::Gas => "gas",
            Self::TwoPhase => "two_phase",
        }
    }

    /// Short name for display, e.g. in a table cell: "two-phase".
    pub const fn label(&self, locale: Locale) -> &'static str {
        locale.pick(match self {
            Self::Liquid => text("liquid", "líquido"),
            Self::Supercritical => text("supercritical", "supercrítico"),
            Self::SupercriticalGas => text("supercritical gas", "gas supercrítico"),
            Self::SupercriticalLiquid => text("supercritical liquid", "líquido supercrítico"),
            Self::CriticalPoint => text("critical point", "punto crítico"),
            Self::Gas => text("gas", "gas"),
            Self::TwoPhase => text("two-phase", "bifásico"),
        })
    }

    /// One-line description (CoolProp's definition).
    pub const fn description(&self, locale: Locale) -> &'static str {
        locale.pick(match self {
            Self::Liquid => text("Subcritical liquid", "Líquido subcrítico"),
            Self::Supercritical => text(
                "Supercritical fluid (p > pc, T > Tc)",
                "Fluido supercrítico (p > pc, T > Tc)",
            ),
            Self::SupercriticalGas => text(
                "Supercritical gas (p < pc, T > Tc)",
                "Gas supercrítico (p < pc, T > Tc)",
            ),
            Self::SupercriticalLiquid => text(
                "Supercritical liquid (p > pc, T < Tc)",
                "Líquido supercrítico (p > pc, T < Tc)",
            ),
            Self::CriticalPoint => text("At the critical point", "En el punto crítico"),
            Self::Gas => text("Subcritical gas", "Gas subcrítico"),
            Self::TwoPhase => text(
                "Liquid-vapor mixture inside the saturation dome",
                "Mezcla líquido-vapor dentro de la campana de saturación",
            ),
        })
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
