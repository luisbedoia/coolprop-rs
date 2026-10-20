use std::ffi::NulError;
use std::fmt;

use crate::locale::Locale;
use crate::plot::PlotProperty;
use crate::{InputKind, Reference, Variant, schema};

/// Display shows the message in English; [`PropsError::message`] gives it
/// in another [`Locale`]. Only coolprop-rs's own texts are translated:
/// CoolProp's ([`Failure::CoolProp`]) are passed on as it wrote them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsError {
    /// The name matches no curated fluid nor any of its aliases.
    UnknownFluid(String),
    /// The request itself is malformed (e.g. an unsupported input pair).
    InvalidInput(Invalid),
    /// CoolProp rejected or failed to solve the request (e.g. a state
    /// outside the EOS range).
    CoolProp(Failure),
}

/// Why a request is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Invalid {
    /// No pair of [`schema::pairs`] is made of these two inputs.
    UnsupportedPair(InputKind, InputKind),
    /// Both axes of a diagram measure the same thing.
    SameAxes(PlotProperty, PlotProperty),
    /// Plot limits that are not finite, positive and increasing, as given.
    PlotLimits(String),
    /// The fluid has no saturated liquid at the reference state's point.
    Reference {
        fluid: String,
        reference: Reference,
        /// CoolProp's reason, untranslated.
        detail: String,
    },
    /// A text that is not translated (e.g. an interior NUL in a name).
    Other(String),
}

/// Why a state or diagram could not be solved.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Failure {
    /// CoolProp's own message, untranslated.
    CoolProp(String),
    /// The solution has zero or negative pressure.
    NonPositivePressure(Variant),
    /// A pseudo-pure mixture solved, without pressure, to a state that is
    /// not in equilibrium (see [`Fluid::state`](crate::Fluid::state)).
    PseudoPurePair(Variant, InputKind, InputKind),
    /// Too few saturation states could be solved to draw the dome.
    NoDome(Variant),
}

impl PropsError {
    /// The message in `locale`; [`Display`](fmt::Display) gives it in English.
    pub fn message(&self, locale: Locale) -> String {
        let es = locale == Locale::Es;
        match self {
            Self::UnknownFluid(name) if es => format!("fluido desconocido: {name}"),
            Self::UnknownFluid(name) => format!("unknown fluid: {name}"),
            Self::InvalidInput(i) if es => format!("entrada no válida: {}", i.message(locale)),
            Self::InvalidInput(i) => format!("invalid input: {}", i.message(locale)),
            Self::CoolProp(c) if es => format!("error de CoolProp: {}", c.message(locale)),
            Self::CoolProp(c) => format!("CoolProp error: {}", c.message(locale)),
        }
    }
}

impl Invalid {
    /// The reason in `locale`, without the "invalid input" lead of
    /// [`PropsError::message`].
    pub fn message(&self, locale: Locale) -> String {
        let es = locale == Locale::Es;
        match self {
            Self::UnsupportedPair(a, b) if es => format!(
                "par de entradas no soportado: {} y {}",
                a.label(locale),
                b.label(locale)
            ),
            Self::UnsupportedPair(a, b) => format!(
                "unsupported input pair: {} and {}",
                a.label(locale),
                b.label(locale)
            ),
            Self::SameAxes(x, y) if es => format!(
                "un diagrama necesita dos propiedades distintas; se dieron {} y {}",
                axis_label(*x, locale),
                axis_label(*y, locale)
            ),
            Self::SameAxes(x, y) => format!(
                "a diagram needs two different properties, got {} and {}",
                axis_label(*x, locale),
                axis_label(*y, locale)
            ),
            Self::PlotLimits(limits) if es => format!("límites de diagrama no válidos: {limits}"),
            Self::PlotLimits(limits) => format!("invalid plot limits: {limits}"),
            Self::Reference {
                fluid,
                reference,
                detail,
            } if es => format!(
                "{} no admite el estado de referencia {}: {detail}",
                fluid_label(fluid, locale),
                reference.name()
            ),
            Self::Reference {
                fluid,
                reference,
                detail,
            } => format!(
                "{} cannot use the {} reference state: {detail}",
                fluid_label(fluid, locale),
                reference.name()
            ),
            Self::Other(text) => text.clone(),
        }
    }
}

impl Failure {
    /// The reason in `locale`, without the "CoolProp error" lead of
    /// [`PropsError::message`].
    pub fn message(&self, locale: Locale) -> String {
        let es = locale == Locale::Es;
        let name = |f: &Variant| schema::display_name(*f, locale);
        match self {
            Self::CoolProp(text) => text.clone(),
            Self::NonPositivePressure(f) if es => format!(
                "ningún estado de {} con presión positiva tiene estas entradas",
                name(f)
            ),
            Self::NonPositivePressure(f) => {
                format!(
                    "no state of {} at positive pressure has these inputs",
                    name(f)
                )
            }
            Self::PseudoPurePair(f, a, b) if es => format!(
                "{} es una mezcla pseudopura: CoolProp no puede resolver este estado con {} y {}; \
                 da la presión y otra propiedad",
                name(f),
                a.label(locale),
                b.label(locale)
            ),
            Self::PseudoPurePair(f, a, b) => format!(
                "{} is a pseudo-pure mixture: CoolProp cannot solve this state from {} and {}; \
                 give the pressure and another property instead",
                name(f),
                a.label(locale),
                b.label(locale)
            ),
            Self::NoDome(f) if es => {
                format!("no se pudo trazar la campana de saturación de {}", name(f))
            }
            Self::NoDome(f) => format!("could not trace the saturation dome of {}", name(f)),
        }
    }
}

fn axis_label(property: PlotProperty, locale: Locale) -> String {
    schema::plot_properties(locale)
        .iter()
        .find(|p| p.name == property)
        .map_or_else(|| format!("{property:?}"), |p| p.description.to_lowercase())
}

/// The display name of a curated fluid; any other name as given.
fn fluid_label(name: &str, locale: Locale) -> String {
    Variant::from_name(name).map_or_else(
        || name.to_owned(),
        |v| schema::display_name(v, locale).to_owned(),
    )
}

impl fmt::Display for PropsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Locale::En))
    }
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Locale::En))
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Locale::En))
    }
}

impl std::error::Error for PropsError {}

impl From<NulError> for PropsError {
    fn from(err: NulError) -> Self {
        Self::InvalidInput(Invalid::Other(err.to_string()))
    }
}
