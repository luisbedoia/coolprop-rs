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

    /// The snake_case name used across the API (`"specific_volume"`, …).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pressure => "pressure",
            Self::Temperature => "temperature",
            Self::Density => "density",
            Self::SpecificVolume => "specific_volume",
            Self::Enthalpy => "enthalpy",
            Self::Entropy => "entropy",
            Self::InternalEnergy => "internal_energy",
        }
    }

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

    /// `"<y>_<x>"`, e.g. `"pressure_enthalpy"` for P–h.
    pub fn id(&self) -> String {
        format!("{}_{}", self.y.property.name(), self.x.property.name())
    }

    /// The catalog diagram with this [`Self::id`].
    pub fn from_id(id: &str) -> Option<Diagram> {
        Self::all().into_iter().find(|d| d.id() == id)
    }

    /// The diagrams engineering texts and charts use, in that order: P–h,
    /// T–s, h–s (Mollier), P–v, T–v and P–T (the phase diagram). Others can
    /// be built with [`Self::new`] but are left out of the catalog: h–u, for
    /// one, collapses onto a line (h = u + pv), and density only repeats
    /// specific volume.
    ///
    /// Scales are the defaults except for P–T, whose pressure is linear as
    /// in phase diagrams: the vapor-pressure curve then rises convex, as
    /// p ≈ e^(A − B/T), instead of flattening out like ln p.
    pub fn all() -> Vec<Diagram> {
        use PlotProperty::*;
        [
            (Enthalpy, Pressure, None),
            (Entropy, Temperature, None),
            (Entropy, Enthalpy, None),
            (SpecificVolume, Pressure, None),
            (SpecificVolume, Temperature, None),
            (Temperature, Pressure, Some(Scale::Linear)),
        ]
        .into_iter()
        .filter_map(|(x, y, y_scale)| {
            let mut d = Diagram::new(x, y).ok()?;
            d.y.scale = y_scale.unwrap_or(d.y.scale);
            Some(d)
        })
        .collect()
    }

    /// Whether `property` is on one of the axes.
    pub fn has(&self, property: PlotProperty) -> bool {
        self.axis(property).is_some()
    }

    /// The axis `property` is on, if any.
    pub fn axis(&self, property: PlotProperty) -> Option<Axis> {
        [self.x, self.y]
            .into_iter()
            .find(|a| a.property == property)
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// The unit values are read in, as an affine map from SI: `shown = si ·
/// scale + offset`. Degrees Celsius are `{scale: 1, offset: -273.15}`,
/// kilopascals `{scale: 1e-3, offset: 0}`. Defaults to SI itself.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DisplayUnit {
    pub scale: f64,
    pub offset: f64,
}

impl DisplayUnit {
    pub const SI: Self = Self {
        scale: 1.0,
        offset: 0.0,
    };

    pub fn show(self, si: f64) -> f64 {
        si * self.scale + self.offset
    }

    pub fn si(self, shown: f64) -> f64 {
        (shown - self.offset) / self.scale
    }
}

impl Default for DisplayUnit {
    fn default() -> Self {
        Self::SI
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

/// Default saturation-dome resolution: states per branch.
pub const DOME_POINTS: usize = 60;

impl<'a> PropertyPlot<'a> {
    /// Plot over [`Limits::default_for`] the fluid.
    pub fn new(fluid: &'a Fluid) -> Result<Self, PropsError> {
        Self::with_limits(fluid, Limits::default_for(fluid))
    }

    /// Plot over `limits`, with the default dome resolution.
    pub fn with_limits(fluid: &'a Fluid, limits: Limits) -> Result<Self, PropsError> {
        Self::with_resolution(fluid, limits, DOME_POINTS)
    }

    /// Plot over `limits`, tracing the dome with about `dome_points` states
    /// per branch (at least 3).
    pub fn with_resolution(
        fluid: &'a Fluid,
        limits: Limits,
        dome_points: usize,
    ) -> Result<Self, PropsError> {
        let ok = limits.t_min.is_finite()
            && limits.p_min > 0.0
            && limits.t_min < limits.t_max
            && limits.p_min < limits.p_max;
        if !ok {
            return Err(PropsError::InvalidInput(format!(
                "invalid plot limits: {limits:?}"
            )));
        }
        let dome = saturation_dome(fluid, dome_points)?;
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

    /// About `count` values for an isoline family to draw on `diagram`,
    /// round in `unit` (50 °C, 200 kPa, …) and evenly spread across the
    /// dome as the diagram shows it. Quality is spread over (0, 1).
    ///
    /// Most families are a uniform grid of round values (steps of 1, 2, 2.5
    /// or 5 × 10ᵏ, or 1-2-5 per decade for log quantities) inside the dome's
    /// span of the property. Some families are better spread by where they
    /// meet the dome, which is far from linear in their own value:
    /// isotherms on a pressure axis and isobars on a temperature axis (flat
    /// inside the dome) and isochores on a pressure axis. Those are spread
    /// evenly along that axis, then each value is rounded to a step small
    /// enough to keep them that way.
    pub fn suggested_values(
        &self,
        kind: InputKind,
        count: usize,
        diagram: &Diagram,
        unit: DisplayUnit,
    ) -> Vec<f64> {
        if count == 0 {
            return Vec::new();
        }
        let property = match kind {
            InputKind::Quality => {
                return (1..=count).map(|i| i as f64 / (count + 1) as f64).collect();
            }
            InputKind::Pressure => PlotProperty::Pressure,
            InputKind::Temperature => PlotProperty::Temperature,
            InputKind::Density => PlotProperty::Density,
            InputKind::Enthalpy => PlotProperty::Enthalpy,
            InputKind::Entropy => PlotProperty::Entropy,
            InputKind::InternalEnergy => PlotProperty::InternalEnergy,
        };
        let logarithmic = property.default_scale() == Scale::Log && unit.offset == 0.0;
        let shown = match self.spread_along_dome(kind, count, diagram) {
            Some(values) => {
                let values: Vec<f64> = values.iter().map(|&v| unit.show(v)).collect();
                round_each(&values, logarithmic && values.iter().all(|&v| v > 0.0))
            }
            None => {
                let Some((lo, hi)) = self.dome_range(property) else {
                    return Vec::new();
                };
                let (lo, hi) = (unit.show(lo), unit.show(hi));
                let (lo, hi) = (lo.min(hi), lo.max(hi));
                round_grid(lo, hi, count, logarithmic && lo > 0.0)
            }
        };
        shown.into_iter().map(|v| unit.si(v)).collect()
    }

    /// `count` values (SI) of a family spread evenly, along an axis of
    /// `diagram`, by where it meets the dome:
    /// - isotherms by saturation pressure on a pressure axis, and isobars by
    ///   saturation temperature on a temperature axis (both lie flat inside
    ///   the dome);
    /// - isochores by the pressure where they leave the dew line, on a
    ///   pressure axis.
    ///
    /// `None` for other families, or if a saturation state cannot be solved.
    fn spread_along_dome(
        &self,
        kind: InputKind,
        count: usize,
        diagram: &Diagram,
    ) -> Option<Vec<f64>> {
        let f = self.fluid;
        let at = |scale: Scale, lo: f64, hi: f64, i: usize| match scale {
            Scale::Log => log(lo, hi, i, count + 2),
            Scale::Linear => lin(lo, hi, i, count + 2),
        };
        // Midway through the dome: a blend's isotherm glides in pressure
        // (and its isobar in temperature) from one branch to the other.
        let mid = 0.5;
        let p_axis = diagram.axis(PlotProperty::Pressure);
        match kind {
            InputKind::Temperature | InputKind::Density if p_axis.is_some() => {
                let (lo, hi) = self.dome_p_span();
                let (q, of): (f64, fn(&State) -> f64) = match kind {
                    InputKind::Temperature => (mid, State::temperature),
                    _ => (1.0, State::density),
                };
                (1..=count)
                    .map(|i| {
                        let p = at(p_axis?.scale, lo, hi, i);
                        f.state(Input::Pressure(p), Input::Quality(q))
                            .ok()
                            .map(|s| of(&s))
                    })
                    .collect()
            }
            InputKind::Pressure => {
                let t_axis = diagram.axis(PlotProperty::Temperature)?;
                let (lo, hi) = self.dome_range(PlotProperty::Temperature)?;
                (1..=count)
                    .map(|i| {
                        let t = at(t_axis.scale, lo, hi, i);
                        let dew = f.state(Input::Temperature(t), Input::Quality(1.0)).ok()?;
                        let bubble = f.state(Input::Temperature(t), Input::Quality(0.0)).ok()?;
                        isotherm_in_dome(f, t, mid, dew.pressure(), bubble.pressure())
                            .map(|s| s.pressure())
                    })
                    .collect()
            }
            _ => None,
        }
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

/// Share of a span kept clear at each end when placing round values: the
/// dome's ends are the triple and critical points, where isolines barely
/// exist.
const EDGE: f64 = 0.04;

/// `count` consecutive points of the coarsest round grid that fits them in
/// `[lo, hi]`, centered in it. Linear grids step by 1, 2, 2.5 or 5 × 10ᵏ;
/// log grids take 1-2-5 per decade, or coarser or finer sequences. Falls
/// back to rounding evenly spread values when no grid fits.
fn round_grid(lo: f64, hi: f64, count: usize, logarithmic: bool) -> Vec<f64> {
    let grid = if logarithmic {
        let (a, b) = (lo.ln(), hi.ln());
        let margin = (b - a) * EDGE;
        log_grid((a + margin).exp(), (b - margin).exp(), count)
    } else {
        let margin = (hi - lo) * EDGE;
        lin_grid(lo + margin, hi - margin, count)
    };
    grid.unwrap_or_else(|| {
        let spread: Vec<f64> = (1..=count)
            .map(|i| match logarithmic {
                true => log(lo, hi, i, count + 2),
                false => lin(lo, hi, i, count + 2),
            })
            .collect();
        round_each(&spread, logarithmic)
    })
}

fn lin_grid(lo: f64, hi: f64, count: usize) -> Option<Vec<f64>> {
    if hi <= lo || !lo.is_finite() || !hi.is_finite() {
        return None;
    }
    let top = (hi - lo).log10().ceil() as i32;
    for k in (top - 12..=top).rev() {
        for m in [5.0, 2.5, 2.0, 1.0] {
            let step = m * 10f64.powi(k);
            let (first, last) = ((lo / step).ceil(), (hi / step).floor());
            let available = last - first + 1.0;
            if available >= count as f64 {
                let start = first + ((available - count as f64) / 2.0).floor();
                return Some(
                    (0..count)
                        .map(|i| (start + i as f64) * step)
                        .map(clean)
                        .collect(),
                );
            }
        }
    }
    None
}

fn log_grid(lo: f64, hi: f64, count: usize) -> Option<Vec<f64>> {
    if !(hi > lo && lo > 0.0) || !hi.is_finite() {
        return None;
    }
    // (mantissas, decade stride), coarsest first.
    const GRIDS: [(&[f64], i32); 8] = [
        (&[1.0], 4),
        (&[1.0], 3),
        (&[1.0], 2),
        (&[1.0], 1),
        (&[1.0, 3.0], 1),
        (&[1.0, 2.0, 5.0], 1),
        (&[1.0, 2.0, 3.0, 5.0, 7.0], 1),
        (&[1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 1),
    ];
    let (d_lo, d_hi) = (lo.log10().floor() as i32, hi.log10().ceil() as i32);
    for (mantissas, stride) in GRIDS {
        let points: Vec<f64> = (d_lo..=d_hi)
            .filter(|d| d.rem_euclid(stride) == 0)
            .flat_map(|d| mantissas.iter().map(move |m| clean(m * 10f64.powi(d))))
            .filter(|v| (lo..=hi).contains(v))
            .collect();
        if points.len() >= count {
            let start = (points.len() - count) / 2;
            return Some(points[start..start + count].to_vec());
        }
    }
    None
}

/// Rounds each value to a round step (1, 2 or 5 × 10ᵏ) no larger than half
/// the gap to its nearest neighbor — measured on a log scale if
/// `logarithmic` — so the values stay distinct and keep their spacing.
fn round_each(values: &[f64], logarithmic: bool) -> Vec<f64> {
    let position = |v: f64| if logarithmic { v.ln() } else { v };
    (0..values.len())
        .map(|i| {
            let v = values[i];
            let gap = [i.checked_sub(1), Some(i + 1)]
                .into_iter()
                .flatten()
                .filter_map(|j| values.get(j))
                .map(|&w| (position(w) - position(v)).abs())
                .fold(f64::INFINITY, f64::min);
            // Alone: two significant digits.
            let tolerance = match (gap.is_finite(), logarithmic) {
                (false, _) => v.abs() / 10.0,
                (true, false) => gap / 2.0,
                (true, true) => v * (1.0 - (-gap / 2.0).exp()),
            };
            if tolerance <= 0.0 || !tolerance.is_finite() || !v.is_finite() {
                return v;
            }
            let step = round_step_below(tolerance);
            clean((v / step).round() * step)
        })
        .collect()
}

/// The largest 1, 2 or 5 × 10ᵏ not above `x` (> 0).
fn round_step_below(x: f64) -> f64 {
    let k = x.log10().floor();
    let base = 10f64.powf(k);
    [5.0, 2.0, 1.0]
        .into_iter()
        .map(|m| m * base)
        .find(|&s| s <= x)
        .unwrap_or(base)
}

/// Drops float noise from a product like 3 × 0.1, so values read as typed.
fn clean(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let digits = 12 - v.abs().log10().floor() as i32;
    let scale = 10f64.powi(digits.clamp(-300, 300));
    (v * scale).round() / scale
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
