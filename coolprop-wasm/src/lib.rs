//! JSON API behind the wasm module, kept free of FFI so it builds and is
//! tested natively. `main.rs` only exposes it as `extern "C"` functions.
//!
//! Every function returns a JSON envelope: `{"ok": <payload>}` or
//! `{"error": {"kind": "unknown_fluid" | "invalid_input" | "coolprop",
//! "message": "..."}}`.
//!
//! Every request may give a `locale`, `"en"` (default) or `"es"`, for
//! display names, schema texts and error messages. Only coolprop-rs's own
//! messages are translated; CoolProp's come as it wrote them.
//!
//! Every request naming a `fluid` may also give a `reference`: `"DEF"`
//! (default), `"IIR"`, `"ASHRAE"` or `"NBP"`, where its enthalpy and entropy
//! are measured from (see [`Reference`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use coolprop::plot::{Axis, Diagram, DisplayUnit, Limits, PlotProperty, PropertyPlot};
use coolprop::schema::{self, DiagramInfo, InputInfo, PhaseInfo, PlotPropertyInfo, PropertyInfo};
use coolprop::{
    CriticalPoint, Fluid, FluidData, Input, InputKind, Locale, PropsError, Reference, State,
    Variant,
};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `{"ok": T}`.
pub fn version() -> String {
    ok(coolprop::version())
}

/// Request `{locale?}` (or `null`) → `{"ok": FluidData[]}`: the curated
/// catalog, in build order, each with its `display_name` in `locale`.
pub fn catalog(request: &str) -> String {
    respond(request, |_: IgnoredAny, locale| {
        Ok(Variant::ALL
            .iter()
            .map(|&v| LocalizedData::new(v, locale))
            .collect::<Vec<_>>())
    })
}

/// Request `{locale?}` (or `null`) → `{"ok": {inputs, pairs, properties, phases,
/// plot_properties, diagrams}}`: the API's own description, with its texts
/// in `locale` (`"en"` by default, or `"es"`), see [`coolprop::schema`].
pub fn schema(request: &str) -> String {
    respond(request, |_: IgnoredAny, locale| {
        Ok(Schema {
            inputs: schema::inputs(locale),
            pairs: schema::pairs().collect(),
            properties: schema::properties(locale),
            phases: schema::phases(locale).collect(),
            plot_properties: schema::plot_properties(locale),
            diagrams: schema::diagrams(),
        })
    })
}

/// Request `{fluid, diagram, isolines?: [{kind, values?, count?, unit?}], points?,
/// dome_points?, limits?}` → `{"ok": DiagramData}`: the axes with their
/// ranges, the saturation dome and the requested isoline families, all
/// projected onto the diagram's axes. Unsolvable points are `null` (a break
/// in the curve).
///
/// The work a request asks for is bounded (see [`MAX_POINTS`],
/// [`MAX_DOME_POINTS`], [`MAX_ISOLINES`], [`MAX_ISOLINE_STATES`]): past the
/// bounds it is rejected as `invalid_input`, so no request can stall the
/// page.
pub fn diagram(request: &str) -> String {
    respond(request, |req: DiagramRequest, _| {
        let d = Diagram::from_id(&req.diagram)
            .ok_or_else(|| RequestError::UnknownDiagram(req.diagram.clone()))?;
        req.check_bounds()?;
        with_fluid(&req.fluid, req.reference, |f| {
            let limits = req.limits.unwrap_or_else(|| Limits::default_for(f));
            let plot = PropertyPlot::with_resolution(f, limits, req.dome_points)?;
            let (x, y) = (d.x.property, d.y.property);
            let project = |states: &[State]| Curve {
                x: states.iter().map(|s| x.of(s)).collect(),
                y: states.iter().map(|s| y.of(s)).collect(),
            };
            let mut isolines = Vec::new();
            for spec in &req.isolines {
                let kind = parse_kind(&spec.kind)?;
                if !d.isoline_kinds().contains(&kind) {
                    return Err(RequestError::IsolinesNotDrawn(kind, d.id()).into());
                }
                let values = match (&spec.values, spec.count) {
                    (Some(values), _) => values.clone(),
                    (None, count) => plot.suggested_values(
                        kind,
                        count.unwrap_or(DEFAULT_COUNT),
                        &d,
                        spec.unit.unwrap_or_default(),
                    ),
                };
                for iso in plot.isolines(kind, &values, req.points) {
                    let (xs, ys) = iso.project(x, y);
                    isolines.push(IsolineCurve {
                        kind,
                        value: iso.value,
                        x: xs,
                        y: ys,
                    });
                }
            }
            Ok(DiagramData {
                id: d.id(),
                x: AxisData::new(d.x, plot.range(x)),
                y: AxisData::new(d.y, plot.range(y)),
                limits: plot.limits(),
                dome: Dome {
                    liquid: project(&plot.dome().liquid),
                    vapor: project(&plot.dome().vapor),
                },
                isolines,
            })
        })
    })
}

/// Request `{fluid, reference?, locale?}` → `{"ok": {data, critical,
/// reference}}`, `data` with its `display_name` in `locale`.
pub fn fluid(request: &str) -> String {
    respond(request, |req: FluidRequest, locale| {
        with_fluid(&req.fluid, req.reference, |f| {
            Ok(FluidInfo {
                data: LocalizedData::new(f.variant(), locale),
                critical: f.critical(),
                reference: f.reference(),
            })
        })
    })
}

/// Request `{fluid, inputs: {<name>: value, <name>: value}}` →
/// `{"ok": State}`.
pub fn state(request: &str) -> String {
    respond(request, |req: StateRequest, _| {
        with_fluid(&req.fluid, req.reference, |f| solve(f, &req.inputs))
    })
}

/// Request `{fluid, inputs: [{..}, ..]}` → `{"ok": [{"ok": State} |
/// {"error": ..}, ..]}`: one envelope per input set, so one bad point does
/// not fail the batch.
pub fn states(request: &str) -> String {
    respond(request, |req: StatesRequest, locale| {
        with_fluid(&req.fluid, req.reference, |f| {
            Ok(req
                .inputs
                .iter()
                .map(|inputs| Envelope::new(solve(f, inputs), locale))
                .collect::<Vec<_>>())
        })
    })
}

#[derive(Deserialize)]
struct FluidRequest {
    fluid: String,
    #[serde(default)]
    reference: Reference,
}

#[derive(Deserialize)]
struct StateRequest {
    fluid: String,
    #[serde(default)]
    reference: Reference,
    inputs: Map<String, Value>,
}

#[derive(Deserialize)]
struct StatesRequest {
    fluid: String,
    #[serde(default)]
    reference: Reference,
    inputs: Vec<Map<String, Value>>,
}

/// Most states per isoline.
pub const MAX_POINTS: usize = 500;
/// Most states per branch of the saturation dome.
pub const MAX_DOME_POINTS: usize = 500;
/// Most isolines per request, across families.
pub const MAX_ISOLINES: usize = 30;
/// Most isoline states per request: isolines × points. At this budget, in
/// desktop Chromium, a request takes ~80 ms for the median fluid and family,
/// ~0.27 s at the 95th percentile and ~1.2 s for the slowest (R143a).
pub const MAX_ISOLINE_STATES: usize = 5000;
/// Suggested isolines per family when the request gives no `count`.
const DEFAULT_COUNT: usize = 5;

#[derive(Deserialize)]
struct DiagramRequest {
    fluid: String,
    #[serde(default)]
    reference: Reference,
    diagram: String,
    #[serde(default)]
    isolines: Vec<IsolineSpec>,
    #[serde(default = "default_points")]
    points: usize,
    #[serde(default = "default_dome_points")]
    dome_points: usize,
    #[serde(default)]
    limits: Option<Limits>,
}

fn default_points() -> usize {
    100
}

fn default_dome_points() -> usize {
    coolprop::plot::DOME_POINTS
}

impl DiagramRequest {
    fn check_bounds(&self) -> Result<(), RequestError> {
        if !(2..=MAX_POINTS).contains(&self.points) {
            return Err(RequestError::Points);
        }
        if !(3..=MAX_DOME_POINTS).contains(&self.dome_points) {
            return Err(RequestError::DomePoints);
        }
        let isolines: usize = self
            .isolines
            .iter()
            .map(|s| match &s.values {
                Some(values) => values.len(),
                None => s.count.unwrap_or(DEFAULT_COUNT),
            })
            .fold(0, usize::saturating_add);
        if isolines > MAX_ISOLINES {
            return Err(RequestError::TooManyIsolines(isolines));
        }
        if isolines.saturating_mul(self.points) > MAX_ISOLINE_STATES {
            return Err(RequestError::TooManyStates {
                isolines,
                points: self.points,
            });
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct IsolineSpec {
    kind: String,
    /// Explicit values; otherwise `count` suggested ones (default 5).
    #[serde(default)]
    values: Option<Vec<f64>>,
    #[serde(default)]
    count: Option<usize>,
    /// The unit suggested values are made round in; SI by default.
    #[serde(default)]
    unit: Option<DisplayUnit>,
}

#[derive(Serialize)]
struct AxisData {
    property: PlotProperty,
    scale: coolprop::plot::Scale,
    /// Span of the property over the plot domain; `null` if it cannot be
    /// determined.
    range: Option<[f64; 2]>,
}

impl AxisData {
    fn new(axis: Axis, range: Option<(f64, f64)>) -> Self {
        Self {
            property: axis.property,
            scale: axis.scale,
            range: range.map(|(lo, hi)| [lo, hi]),
        }
    }
}

/// Coordinates on the diagram's axes; `NaN` serializes as `null`.
#[derive(Serialize)]
struct Curve {
    x: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Serialize)]
struct Dome {
    liquid: Curve,
    vapor: Curve,
}

#[derive(Serialize)]
struct IsolineCurve {
    kind: InputKind,
    value: f64,
    x: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Serialize)]
struct DiagramData {
    id: String,
    x: AxisData,
    y: AxisData,
    limits: Limits,
    dome: Dome,
    isolines: Vec<IsolineCurve>,
}

#[derive(Serialize)]
struct Schema {
    inputs: &'static [InputInfo],
    pairs: Vec<(InputKind, InputKind)>,
    properties: &'static [PropertyInfo],
    phases: Vec<PhaseInfo>,
    plot_properties: &'static [PlotPropertyInfo],
    diagrams: Vec<DiagramInfo>,
}

/// A fluid's catalog constants and its name for display.
#[derive(Serialize)]
struct LocalizedData {
    #[serde(flatten)]
    data: &'static FluidData,
    display_name: &'static str,
}

impl LocalizedData {
    fn new(fluid: Variant, locale: Locale) -> Self {
        Self {
            data: fluid.data(),
            display_name: schema::display_name(fluid, locale),
        }
    }
}

#[derive(Serialize)]
struct FluidInfo {
    data: LocalizedData,
    critical: CriticalPoint,
    reference: Reference,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Envelope<T> {
    Ok(T),
    Error(ErrorBody),
}

impl<T> Envelope<T> {
    fn new(result: Result<T, ApiError>, locale: Locale) -> Self {
        match result {
            Ok(v) => Self::Ok(v),
            Err(e) => Self::Error(ErrorBody::new(e, locale)),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    kind: &'static str,
    message: String,
}

impl ErrorBody {
    fn new(e: ApiError, locale: Locale) -> Self {
        match e {
            ApiError::Props(e) => Self {
                kind: match e {
                    PropsError::UnknownFluid(_) => "unknown_fluid",
                    PropsError::InvalidInput(_) => "invalid_input",
                    PropsError::CoolProp(_) => "coolprop",
                },
                message: e.message(locale),
            },
            ApiError::Request(e) => Self {
                kind: "invalid_input",
                message: match locale {
                    Locale::Es => format!("entrada no válida: {}", e.message(locale)),
                    _ => format!("invalid input: {}", e.message(locale)),
                },
            },
        }
    }
}

/// Why a request failed: in coolprop, or in the request itself.
#[derive(Debug)]
enum ApiError {
    Props(PropsError),
    Request(RequestError),
}

impl From<PropsError> for ApiError {
    fn from(e: PropsError) -> Self {
        Self::Props(e)
    }
}

impl From<RequestError> for ApiError {
    fn from(e: RequestError) -> Self {
        Self::Request(e)
    }
}

/// A request this API cannot take; reported as `invalid_input`.
#[derive(Debug)]
enum RequestError {
    /// Not JSON, or not the request's shape; serde's text, untranslated.
    Malformed(String),
    UnknownDiagram(String),
    IsolinesNotDrawn(InputKind, String),
    Points,
    DomePoints,
    TooManyIsolines(usize),
    TooManyStates {
        isolines: usize,
        points: usize,
    },
    InputCount(usize),
    UnknownInput(String),
    NotANumber(String),
}

impl RequestError {
    fn message(&self, locale: Locale) -> String {
        let es = locale == Locale::Es;
        let known = || {
            let names: Vec<&str> = InputKind::ALL.iter().map(|k| k.name()).collect();
            names.join(", ")
        };
        match self {
            Self::Malformed(e) if es => format!("solicitud mal formada: {e}"),
            Self::Malformed(e) => format!("malformed request: {e}"),
            Self::UnknownDiagram(id) if es => format!("diagrama desconocido: {id}"),
            Self::UnknownDiagram(id) => format!("unknown diagram: {id}"),
            Self::IsolinesNotDrawn(kind, id) if es => format!(
                "las isolíneas de {} no se dibujan en {id}",
                kind.label(locale)
            ),
            Self::IsolinesNotDrawn(kind, id) => {
                format!("{} isolines are not drawn on {id}", kind.label(locale))
            }
            Self::Points if es => format!("points debe estar entre 2 y {MAX_POINTS}"),
            Self::Points => format!("points must be between 2 and {MAX_POINTS}"),
            Self::DomePoints if es => {
                format!("dome_points debe estar entre 3 y {MAX_DOME_POINTS}")
            }
            Self::DomePoints => format!("dome_points must be between 3 and {MAX_DOME_POINTS}"),
            Self::TooManyIsolines(n) if es => {
                format!("como máximo {MAX_ISOLINES} isolíneas por solicitud; se pidieron {n}")
            }
            Self::TooManyIsolines(n) => {
                format!("at most {MAX_ISOLINES} isolines per request, got {n}")
            }
            Self::TooManyStates { isolines, points } if es => format!(
                "como máximo {MAX_ISOLINE_STATES} estados de isolíneas (isolíneas × points) \
                 por solicitud; se pidieron {isolines} × {points} = {}",
                isolines * points
            ),
            Self::TooManyStates { isolines, points } => format!(
                "at most {MAX_ISOLINE_STATES} isoline states (isolines × points) \
                 per request, got {isolines} × {points} = {}",
                isolines * points
            ),
            Self::InputCount(n) if es => {
                format!("se esperaban exactamente dos entradas; se recibieron {n}")
            }
            Self::InputCount(n) => format!("expected exactly two inputs, got {n}"),
            Self::UnknownInput(name) if es => format!(
                "entrada desconocida `{name}`; se esperaba una de {}",
                known()
            ),
            Self::UnknownInput(name) => {
                format!("unknown input `{name}`; expected one of {}", known())
            }
            Self::NotANumber(name) if es => format!("la entrada `{name}` debe ser un número"),
            Self::NotANumber(name) => format!("input `{name}` must be a number"),
        }
    }
}

fn ok<T: Serialize>(payload: T) -> String {
    to_json(&Envelope::<T>::Ok(payload))
}

/// Parses `request`, runs `f` with it and its `locale`, and wraps the
/// outcome in an envelope, any error message in that locale.
fn respond<Req, T>(request: &str, f: impl FnOnce(Req, Locale) -> Result<T, ApiError>) -> String
where
    Req: for<'de> Deserialize<'de>,
    T: Serialize,
{
    let mut locale = Locale::default();
    let result = parse(request, &mut locale).and_then(|req| f(req, locale));
    to_json(&Envelope::new(result, locale))
}

/// Reads `locale` first, so that a malformed rest is reported in it.
fn parse<Req>(request: &str, locale: &mut Locale) -> Result<Req, ApiError>
where
    Req: for<'de> Deserialize<'de>,
{
    let malformed = |e: serde_json::Error| RequestError::Malformed(e.to_string());
    let value: Value = serde_json::from_str(request).map_err(malformed)?;
    if let Some(l) = value.get("locale") {
        *locale = Locale::deserialize(l).map_err(malformed)?;
    }
    Ok(serde_json::from_value(value).map_err(malformed)?)
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|e| {
        // Only non-finite floats could fail, and payloads never carry them.
        format!(r#"{{"error":{{"kind":"coolprop","message":"serialize failed: {e}"}}}}"#)
    })
}

thread_local! {
    /// One open `Fluid` per curated fluid and reference state, reused
    /// across calls: opening an `AbstractState` costs far more than solving
    /// a state with it. The catalog and the four references bound the size;
    /// wasm runs on a single thread.
    static FLUIDS: RefCell<HashMap<(Variant, Reference), Rc<Fluid>>> =
        RefCell::new(HashMap::new());
}

/// Runs `f` with the cached session for `name` (canonical name or alias)
/// and `reference`, opening it on first use.
fn with_fluid<T>(
    name: &str,
    reference: Reference,
    f: impl FnOnce(&Fluid) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let variant = Variant::from_name(name).ok_or_else(|| PropsError::UnknownFluid(name.into()))?;
    let key = (variant, reference);
    let cached = FLUIDS.with_borrow(|m| m.get(&key).cloned());
    let fluid = match cached {
        Some(fluid) => fluid,
        None => {
            let fluid = Rc::new(Fluid::with_reference(variant, reference)?);
            FLUIDS.with_borrow_mut(|m| m.insert(key, Rc::clone(&fluid)));
            fluid
        }
    };
    f(&fluid)
}

fn solve(fluid: &Fluid, inputs: &Map<String, Value>) -> Result<State, ApiError> {
    let mut parsed = inputs.iter().map(|(k, v)| parse_input(k, v));
    match (parsed.next(), parsed.next(), parsed.next()) {
        (Some(a), Some(b), None) => Ok(fluid.state(a?, b?)?),
        _ => Err(RequestError::InputCount(inputs.len()).into()),
    }
}

fn parse_kind(name: &str) -> Result<InputKind, RequestError> {
    InputKind::from_name(name).ok_or_else(|| RequestError::UnknownInput(name.to_owned()))
}

fn parse_input(name: &str, value: &Value) -> Result<Input, RequestError> {
    let kind = parse_kind(name)?;
    let v = value
        .as_f64()
        .ok_or_else(|| RequestError::NotANumber(name.to_owned()))?;
    Ok(kind.with_value(v))
}
