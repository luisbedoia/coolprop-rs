//! Data for thermodynamic diagrams: the saturation dome and isolines, as
//! solved states that any pair of axes can project. Nothing is drawn here.
//!
//! An isoline holds one [`InputKind`] fixed and sweeps another one that
//! CoolProp can solve together with it; a [`Diagram`] is any two
//! [`PlotProperty`] axes. Keeping them independent is what makes every
//! isoline family available on every diagram.
//!
//! ```no_run
//! use coolprop::plot::{Diagram, PlotProperty, PropertyPlot};
//! use coolprop::{Fluid, InputKind, Variant};
//! let water = Fluid::new(Variant::Water)?;
//! let plot = PropertyPlot::new(&water)?;
//! let ph = Diagram::new(PlotProperty::Enthalpy, PlotProperty::Pressure)?;
//! let isotherm = plot.isoline(InputKind::Temperature, 400.0, 100);
//! let (h, p) = isotherm.project(ph.x.property, ph.y.property);
//! # Ok::<(), coolprop::PropsError>(())
//! ```

use crate::error::PropsError;
use crate::fluid::Fluid;
use crate::input::{Input, InputKind};
use crate::state::State;

/// A property that can be plotted on an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PlotProperty {
    /// Pa
    Pressure,
    /// K
    Temperature,
    /// kg/m³
    Density,
    /// m³/kg
    SpecificVolume,
    /// J/kg
    Enthalpy,
    /// J/(kg·K)
    Entropy,
    /// J/kg
    InternalEnergy,
}

impl PlotProperty {
    pub const ALL: [PlotProperty; 7] = [
        Self::Pressure,
        Self::Temperature,
        Self::Density,
        Self::SpecificVolume,
        Self::Enthalpy,
        Self::Entropy,
        Self::InternalEnergy,
    ];

    /// Reads this property from a solved state.
    pub fn of(self, s: &State) -> f64 {
        match self {
            Self::Pressure => s.pressure(),
            Self::Temperature => s.temperature(),
            Self::Density => s.density(),
            Self::SpecificVolume => 1.0 / s.density(),
            Self::Enthalpy => s.enthalpy(),
            Self::Entropy => s.entropy(),
            Self::InternalEnergy => s.internal_energy(),
        }
    }

    /// Log for quantities spanning orders of magnitude, linear otherwise.
    pub const fn default_scale(self) -> Scale {
        match self {
            Self::Pressure | Self::Density | Self::SpecificVolume => Scale::Log,
            _ => Scale::Linear,
        }
    }

    /// The input this property fixes when held constant (specific volume
    /// fixes density).
    const fn input_kind(self) -> InputKind {
        match self {
            Self::Pressure => InputKind::Pressure,
            Self::Temperature => InputKind::Temperature,
            Self::Density | Self::SpecificVolume => InputKind::Density,
            Self::Enthalpy => InputKind::Enthalpy,
            Self::Entropy => InputKind::Entropy,
            Self::InternalEnergy => InputKind::InternalEnergy,
        }
    }

    /// Conventional vertical-axis priority: P on top of everything, then T,
    /// then h (Mollier), … Used to orient the diagram catalog.
    const fn y_priority(self) -> u8 {
        match self {
            Self::Pressure => 0,
            Self::Temperature => 1,
            Self::Enthalpy => 2,
            Self::InternalEnergy => 3,
            Self::Density => 4,
            Self::SpecificVolume => 5,
            Self::Entropy => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Scale {
    Linear,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Axis {
    pub property: PlotProperty,
    pub scale: Scale,
}

/// A pair of axes, e.g. pressure–enthalpy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Diagram {
    pub x: Axis,
    pub y: Axis,
}

impl Diagram {
    /// Diagram with default scales. Errors if both axes measure the same
    /// thing (including density against specific volume).
    pub fn new(x: PlotProperty, y: PlotProperty) -> Result<Self, PropsError> {
        if x.input_kind() == y.input_kind() {
            return Err(PropsError::InvalidInput(format!(
                "a diagram needs two different properties, got {x:?} and {y:?}"
            )));
        }
        Ok(Self {
            x: Axis {
                property: x,
                scale: x.default_scale(),
            },
            y: Axis {
                property: y,
                scale: y.default_scale(),
            },
        })
    }

    /// Every distinct diagram, conventionally oriented (P–h, T–s, h–s, P–T,
    /// P–v, …): 20 of them.
    pub fn all() -> Vec<Diagram> {
        let mut out = Vec::new();
        for (i, &a) in PlotProperty::ALL.iter().enumerate() {
            for &b in &PlotProperty::ALL[i + 1..] {
                let (y, x) = if a.y_priority() <= b.y_priority() {
                    (a, b)
                } else {
                    (b, a)
                };
                if let Ok(d) = Diagram::new(x, y) {
                    out.push(d);
                }
            }
        }
        out
    }

    /// Isoline families worth drawing on this diagram: every input except
    /// the axes themselves (those would be grid lines) and, on P–T, quality
    /// (every quality collapses onto the vapor-pressure curve).
    pub fn isoline_kinds(&self) -> Vec<InputKind> {
        let axes = [self.x.property.input_kind(), self.y.property.input_kind()];
        let pt = axes.contains(&InputKind::Pressure) && axes.contains(&InputKind::Temperature);
        InputKind::ALL
            .into_iter()
            .filter(|k| !axes.contains(k))
            .filter(|k| !(pt && *k == InputKind::Quality))
            .collect()
    }
}

/// Temperature/pressure domain the isolines sweep and the axes span.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Limits {
    /// K
    pub t_min: f64,
    /// K
    pub t_max: f64,
    /// Pa
    pub p_min: f64,
    /// Pa
    pub p_max: f64,
}

impl Limits {
    /// CoolProp's default plot domain: `[1.01·T_triple, 2.25·T_c]` ×
    /// `[1.01·p_triple, 2.25·p_c]`, clipped to the EOS limits.
    pub fn default_for(fluid: &Fluid) -> Self {
        let (data, crit) = (fluid.data(), fluid.critical());
        Self {
            t_min: data.t_triple * 1.01,
            t_max: (crit.temperature * 2.25).min(data.t_max * 0.999),
            p_min: data.p_triple * 1.01,
            p_max: (crit.pressure * 2.25).min(data.p_max * 0.999),
        }
    }
}

/// The two-phase boundary from the triple point to the critical point.
/// Both branches end at the same critical state, closing the dome.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SaturationDome {
    /// Saturated liquid (bubble line), triple → critical.
    pub liquid: Vec<State>,
    /// Saturated vapor (dew line), triple → critical.
    pub vapor: Vec<State>,
}

/// One isoline: a sweep of states that share `value` of `kind`. A point
/// CoolProp cannot solve is `None`, i.e. a break in the curve.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Isoline {
    pub kind: InputKind,
    pub value: f64,
    pub states: Vec<Option<State>>,
}

impl Isoline {
    /// Coordinates on the given axes; unsolved points become `NaN`.
    pub fn project(&self, x: PlotProperty, y: PlotProperty) -> (Vec<f64>, Vec<f64>) {
        self.states
            .iter()
            .map(|s| s.map_or((f64::NAN, f64::NAN), |s| (x.of(&s), y.of(&s))))
            .unzip()
    }
}

/// Plot data for one fluid: its domain, its saturation dome (computed once)
/// and isolines on demand.
#[derive(Debug)]
pub struct PropertyPlot<'a> {
    fluid: &'a Fluid,
    limits: Limits,
    dome: SaturationDome,
}

/// Default saturation-dome resolution.
const DOME_POINTS: usize = 60;

impl<'a> PropertyPlot<'a> {
    /// Plot over [`Limits::default_for`] the fluid.
    pub fn new(fluid: &'a Fluid) -> Result<Self, PropsError> {
        Self::with_limits(fluid, Limits::default_for(fluid))
    }

    pub fn with_limits(fluid: &'a Fluid, limits: Limits) -> Result<Self, PropsError> {
        let ok = limits.t_min.is_finite()
            && limits.p_min > 0.0
            && limits.t_min < limits.t_max
            && limits.p_min < limits.p_max;
        if !ok {
            return Err(PropsError::InvalidInput(format!(
                "invalid plot limits: {limits:?}"
            )));
        }
        let dome = saturation_dome(fluid, DOME_POINTS)?;
        Ok(Self {
            fluid,
            limits,
            dome,
        })
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    pub fn dome(&self) -> &SaturationDome {
        &self.dome
    }

    /// Span of `property` over the plot domain (the T–P rectangle's edges
    /// plus the dome): a default axis range.
    pub fn range(&self, property: PlotProperty) -> Option<(f64, f64)> {
        let l = self.limits;
        let n = 24;
        let mut states: Vec<State> = Vec::new();
        for i in 0..n {
            let t = lin(l.t_min, l.t_max, i, n);
            let p = log(l.p_min, l.p_max, i, n);
            for (a, b) in [
                (Input::Pressure(l.p_min), Input::Temperature(t)),
                (Input::Pressure(l.p_max), Input::Temperature(t)),
                (Input::Pressure(p), Input::Temperature(l.t_min)),
                (Input::Pressure(p), Input::Temperature(l.t_max)),
            ] {
                states.extend(self.fluid.state(a, b).ok());
            }
        }
        states.extend(self.dome.liquid.iter().chain(&self.dome.vapor).copied());
        min_max(states.iter().map(|s| property.of(s)))
    }

    /// Span of `property` along the saturation dome: the region where
    /// isolines are most informative, a default range for isoline values.
    pub fn dome_range(&self, property: PlotProperty) -> Option<(f64, f64)> {
        min_max(
            self.dome
                .liquid
                .iter()
                .chain(&self.dome.vapor)
                .map(|s| property.of(s)),
        )
    }

    /// `count` values for an isoline family, evenly spread inside (not at
    /// the edges of) the dome's span of that property — at the edges sit the
    /// triple and critical points, where isolines barely exist. Quality
    /// spans (0, 1); pressure and density are log spaced.
    pub fn suggested_values(&self, kind: InputKind, count: usize) -> Vec<f64> {
        if count == 0 {
            return Vec::new();
        }
        if kind == InputKind::Quality {
            return (1..=count).map(|i| i as f64 / (count + 1) as f64).collect();
        }
        let property = match kind {
            InputKind::Pressure => PlotProperty::Pressure,
            InputKind::Temperature => PlotProperty::Temperature,
            InputKind::Density => PlotProperty::Density,
            InputKind::Enthalpy => PlotProperty::Enthalpy,
            InputKind::Entropy => PlotProperty::Entropy,
            InputKind::InternalEnergy => PlotProperty::InternalEnergy,
            InputKind::Quality => unreachable!(),
        };
        let Some((lo, hi)) = self.dome_range(property) else {
            return Vec::new();
        };
        let logarithmic = property.default_scale() == Scale::Log && lo > 0.0;
        (1..=count)
            .map(|i| match logarithmic {
                true => log(lo, hi, i, count + 2),
                false => lin(lo, hi, i, count + 2),
            })
            .collect()
    }

    /// Pressures the dome spans below the critical point.
    fn dome_p_span(&self) -> (f64, f64) {
        let below_apex = |branch: &[State]| {
            let crit = self.fluid.critical();
            branch
                .iter()
                .filter(|s| s.temperature() < crit.temperature)
                .map(|s| s.pressure())
                .collect::<Vec<_>>()
        };
        let pressures = [below_apex(&self.dome.liquid), below_apex(&self.dome.vapor)].concat();
        min_max(pressures.into_iter()).unwrap_or((f64::NAN, f64::NAN))
    }

    /// The isoline of `kind` = `value`, sampled with about `points` states.
    pub fn isoline(&self, kind: InputKind, value: f64, points: usize) -> Isoline {
        let points = points.max(2);
        let l = self.limits;
        let f = self.fluid;
        let crit = f.critical();
        let solve = |a: Input, b: Input| f.state(a, b).ok();
        let states = match kind {
            InputKind::Pressure if value < crit.pressure => {
                // Subcooled → dome (by quality: T is not monotonic enough
                // there, and for pure fluids it is constant) → superheated.
                split_sweep(points, |phase| match phase {
                    Phase3::Before(n) => {
                        let t_bub = solve(Input::Pressure(value), Input::Quality(0.0))
                            .map_or(l.t_min, |s| s.temperature());
                        sweep_lin(l.t_min, t_bub - margin_t(t_bub), n, |t| {
                            solve(Input::Pressure(value), Input::Temperature(t))
                        })
                    }
                    Phase3::Dome(n) => sweep_lin(0.0, 1.0, n, |q| {
                        solve(Input::Pressure(value), Input::Quality(q))
                    }),
                    Phase3::After(n) => {
                        let t_dew = solve(Input::Pressure(value), Input::Quality(1.0))
                            .map_or(l.t_max, |s| s.temperature());
                        sweep_lin(t_dew + margin_t(t_dew), l.t_max, n, |t| {
                            solve(Input::Pressure(value), Input::Temperature(t))
                        })
                    }
                })
            }
            InputKind::Temperature if value < crit.temperature => {
                // Superheated (low p) → dome → compressed liquid.
                let dew = solve(Input::Temperature(value), Input::Quality(1.0));
                let bubble = solve(Input::Temperature(value), Input::Quality(0.0));
                split_sweep(points, |phase| match phase {
                    Phase3::Before(n) => {
                        let p_dew = dew.map_or(l.p_min, |s| s.pressure());
                        sweep_log(l.p_min, p_dew * (1.0 - 1e-4), n, |p| {
                            solve(Input::Pressure(p), Input::Temperature(value))
                        })
                    }
                    // Swept by quality, vapor → liquid. For a pure fluid the
                    // pressure is the saturation pressure throughout; a
                    // pseudo-pure blend glides from its dew to its bubble
                    // pressure, so each point is solved for the pressure
                    // where (p, x) has this temperature.
                    Phase3::Dome(n) => match (dew, bubble) {
                        (Some(dew), Some(bubble)) => sweep_lin(1.0, 0.0, n, |x| {
                            isotherm_in_dome(f, value, x, dew.pressure(), bubble.pressure())
                        }),
                        _ => Vec::new(),
                    },
                    Phase3::After(n) => {
                        let p_bub = bubble.map_or(l.p_max, |s| s.pressure());
                        sweep_log(p_bub * (1.0 + 1e-4), l.p_max, n, |p| {
                            solve(Input::Pressure(p), Input::Temperature(value))
                        })
                    }
                })
            }
            InputKind::Pressure | InputKind::Density => {
                let fixed = kind.with_value(value);
                sweep_lin(l.t_min, l.t_max, points, |t| {
                    solve(fixed, Input::Temperature(t))
                })
            }
            InputKind::Temperature
            | InputKind::Enthalpy
            | InputKind::Entropy
            | InputKind::InternalEnergy => {
                let fixed = kind.with_value(value);
                sweep_log(l.p_min, l.p_max, points, |p| {
                    solve(Input::Pressure(p), fixed)
                })
            }
            InputKind::Quality if (0.0..=1.0).contains(&value) => {
                // Swept by pressure: (T, x) with 0 < x < 1 is not solvable
                // for pseudo-pure blends (their bubble and dew temperatures
                // differ), (p, x) is for every fluid.
                let (p_lo, p_hi) = self.dome_p_span();
                sweep_log(p_lo, p_hi, points, |p| {
                    solve(Input::Pressure(p), Input::Quality(value))
                })
            }
            InputKind::Quality => Vec::new(),
        };
        Isoline {
            kind,
            value,
            states,
        }
    }

    /// One isoline per value.
    pub fn isolines(&self, kind: InputKind, values: &[f64], points: usize) -> Vec<Isoline> {
        values
            .iter()
            .map(|&v| self.isoline(kind, v, points))
            .collect()
    }
}

/// Samples the dome with up to `points` states per branch: saturation
/// states bunched toward the critical end, where the dome narrows fastest,
/// plus the critical point itself, solved directly from (ρc, Tc) instead of
/// an ill-conditioned two-phase flash. A temperature where either branch
/// does not converge (it happens near the critical point of some blends) is
/// skipped, as is the apex if it cannot be solved.
fn saturation_dome(fluid: &Fluid, points: usize) -> Result<SaturationDome, PropsError> {
    let (t_lo, t_hi) = dome_t_span(fluid);
    let n = points.max(3) - 1;
    let mut liquid = Vec::with_capacity(n + 1);
    let mut vapor = Vec::with_capacity(n + 1);
    for i in 0..n {
        let frac = i as f64 / (n - 1) as f64;
        let t = t_lo + (t_hi - t_lo) * (1.0 - (1.0 - frac).powf(2.5));
        let liq = fluid.state(Input::Quality(0.0), Input::Temperature(t));
        let vap = fluid.state(Input::Quality(1.0), Input::Temperature(t));
        if let (Ok(liq), Ok(vap)) = (liq, vap) {
            liquid.push(liq);
            vapor.push(vap);
        }
    }
    if liquid.len() < 2 {
        return Err(PropsError::CoolProp(format!(
            "could not trace the saturation dome of {}",
            fluid.data().name
        )));
    }
    let crit = fluid.critical();
    if let Ok(apex) = fluid.state(
        Input::Density(crit.density),
        Input::Temperature(crit.temperature),
    ) {
        liquid.push(apex);
        vapor.push(apex);
    }
    Ok(SaturationDome { liquid, vapor })
}

/// The two-phase state of quality `x` at temperature `t`, between the dew
/// pressure `p_dew` (x = 1) and the bubble pressure `p_bub` (x = 0).
///
/// (T, x) with 0 < x < 1 cannot be solved for pseudo-pure blends, so this
/// solves (p, x) for the pressure where the temperature is `t`: at fixed x,
/// T(p, x) rises with p and brackets `t` between the two pressures. Regula
/// falsi (Illinois) on ln p; for a pure fluid both pressures coincide.
fn isotherm_in_dome(fluid: &Fluid, t: f64, x: f64, p_dew: f64, p_bub: f64) -> Option<State> {
    let at = |p: f64| fluid.state(Input::Pressure(p), Input::Quality(x)).ok();
    if (p_bub / p_dew - 1.0).abs() < 1e-9 {
        return at(p_dew);
    }
    let (mut a, mut b) = (p_dew.ln(), p_bub.ln());
    let (mut sa, mut sb) = (at(p_dew)?, at(p_bub)?);
    let (mut fa, mut fb) = (sa.temperature() - t, sb.temperature() - t);
    if fa * fb > 0.0 {
        return None;
    }
    let tol = t * 1e-10;
    for _ in 0..60 {
        if fa.abs() <= tol {
            return Some(sa);
        }
        if fb.abs() <= tol {
            return Some(sb);
        }
        let c = (a * fb - b * fa) / (fb - fa);
        let sc = at(c.exp())?;
        let fc = sc.temperature() - t;
        if fc * fb < 0.0 {
            (a, fa, sa) = (b, fb, sb);
        } else {
            fa /= 2.0; // Illinois: halve the stale end so it keeps moving
        }
        (b, fb, sb) = (c, fc, sc);
    }
    (fb.abs() <= tol * 1e3).then_some(sb)
}

/// Temperatures where saturation states can be solved: just above the
/// triple point to just below the critical point.
fn dome_t_span(fluid: &Fluid) -> (f64, f64) {
    let tc = fluid.critical().temperature;
    (fluid.data().t_triple * 1.001, tc - (tc * 1e-5).max(0.01))
}

/// Stays clear of the saturation temperature, where (p, T) is ambiguous.
fn margin_t(t: f64) -> f64 {
    (t * 1e-5).max(0.05)
}

enum Phase3 {
    Before(usize),
    Dome(usize),
    After(usize),
}

/// Splits `points` into three roughly equal sweeps and concatenates them.
fn split_sweep(
    points: usize,
    mut sweep: impl FnMut(Phase3) -> Vec<Option<State>>,
) -> Vec<Option<State>> {
    let third = (points / 3).max(2);
    let rest = points.saturating_sub(2 * third).max(2);
    let mut out = sweep(Phase3::Before(third));
    out.extend(sweep(Phase3::Dome(third)));
    out.extend(sweep(Phase3::After(rest)));
    out
}

fn sweep_lin(
    lo: f64,
    hi: f64,
    n: usize,
    f: impl FnMut(f64) -> Option<State>,
) -> Vec<Option<State>> {
    if !(lo.is_finite() && hi.is_finite()) || n == 0 {
        return Vec::new();
    }
    (0..n).map(|i| lin(lo, hi, i, n)).map(f).collect()
}

fn sweep_log(
    lo: f64,
    hi: f64,
    n: usize,
    f: impl FnMut(f64) -> Option<State>,
) -> Vec<Option<State>> {
    if !(lo > 0.0 && hi > 0.0 && hi.is_finite()) || n == 0 || lo >= hi {
        return Vec::new();
    }
    (0..n).map(|i| log(lo, hi, i, n)).map(f).collect()
}

fn lin(lo: f64, hi: f64, i: usize, n: usize) -> f64 {
    if n < 2 {
        return lo;
    }
    lo + (hi - lo) * i as f64 / (n - 1) as f64
}

fn log(lo: f64, hi: f64, i: usize, n: usize) -> f64 {
    if n < 2 {
        return lo;
    }
    lo * (hi / lo).powf(i as f64 / (n - 1) as f64)
}

fn min_max(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    values
        .filter(|v| v.is_finite())
        .fold(None, |acc, v| match acc {
            None => Some((v, v)),
            Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
        })
        .filter(|(lo, hi)| hi > lo)
}
