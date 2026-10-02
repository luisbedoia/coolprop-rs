use std::fmt;

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

    /// One-line English description (CoolProp's definition).
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Liquid => "Subcritical liquid",
            Self::Supercritical => "Supercritical fluid (p > pc, T > Tc)",
            Self::SupercriticalGas => "Supercritical gas (p < pc, T > Tc)",
            Self::SupercriticalLiquid => "Supercritical liquid (p > pc, T < Tc)",
            Self::CriticalPoint => "At the critical point",
            Self::Gas => "Subcritical gas",
            Self::TwoPhase => "Liquid-vapor mixture inside the saturation dome",
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
