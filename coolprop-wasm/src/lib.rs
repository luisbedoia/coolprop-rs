//! JSON API behind the wasm module, kept free of FFI so it builds and is
//! tested natively. `main.rs` only exposes it as `extern "C"` functions.
//!
//! Every function returns a JSON envelope: `{"ok": <payload>}` or
//! `{"error": {"kind": "unknown_fluid" | "invalid_input" | "coolprop",
//! "message": "..."}}`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use coolprop::plot::{Axis, Diagram, Limits, PlotProperty, PropertyPlot};
use coolprop::schema::{self, DiagramInfo, InputInfo, PhaseInfo, PlotPropertyInfo, PropertyInfo};
use coolprop::{CriticalPoint, Fluid, FluidData, Input, InputKind, PropsError, State, Variant};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `{"ok": T}`.
pub fn version() -> String {
    ok(coolprop::version())
}

/// `{"ok": FluidData[]}`: the curated catalog, in build order.
pub fn catalog() -> String {
    let fluids: Vec<&FluidData> = Variant::ALL.iter().map(|v| v.data()).collect();
    ok(fluids)
}

/// `{"ok": {inputs, pairs, properties, phases}}`: the API's own description,
/// see [`coolprop::schema`].
pub fn schema() -> String {
    ok(Schema {
        inputs: schema::inputs(),
        pairs: schema::pairs().collect(),
        properties: schema::properties(),
        phases: schema::phases().collect(),
        plot_properties: schema::plot_properties(),
        diagrams: schema::diagrams(),
    })
}

/// Request `{fluid, diagram, isolines?: [{kind, values?, count?}], points?,
/// limits?}` → `{"ok": DiagramData}`: the axes with their ranges, the
/// saturation dome and the requested isoline families, all projected onto
/// the diagram's axes. Unsolvable points are `null` (a break in the curve).
pub fn diagram(request: &str) -> String {
    respond(request, |req: DiagramRequest| {
        let d = Diagram::from_id(&req.diagram)
            .ok_or_else(|| PropsError::InvalidInput(format!("unknown diagram: {}", req.diagram)))?;
        if !(2..=MAX_POINTS).contains(&req.points) {
            return Err(PropsError::InvalidInput(format!(
                "points must be between 2 and {MAX_POINTS}"
            )));
        }
        with_fluid(&req.fluid, |f| {
            let plot = match req.limits {
                Some(limits) => PropertyPlot::with_limits(f, limits)?,
                None => PropertyPlot::new(f)?,
            };
            let (x, y) = (d.x.property, d.y.property);
            let project = |states: &[State]| Curve {
                x: states.iter().map(|s| x.of(s)).collect(),
                y: states.iter().map(|s| y.of(s)).collect(),
            };
            let mut isolines = Vec::new();
            for spec in &req.isolines {
                let kind = parse_kind(&spec.kind)?;
                if !d.isoline_kinds().contains(&kind) {
                    return Err(PropsError::InvalidInput(format!(
                        "{} isolines are not drawn on {}",
                        kind.name(),
                        d.id()
                    )));
                }
                let values = match (&spec.values, spec.count) {
                    (Some(values), _) => values.clone(),
                    (None, count) => plot.suggested_values(kind, count.unwrap_or(5).min(50)),
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

/// Request `{fluid}` → `{"ok": {data, critical}}`.
pub fn fluid(request: &str) -> String {
    respond(request, |req: FluidRequest| {
        with_fluid(&req.fluid, |f| {
            Ok(FluidInfo {
                data: f.data(),
                critical: f.critical(),
            })
        })
    })
}

/// Request `{fluid, inputs: {<name>: value, <name>: value}}` →
/// `{"ok": State}`.
pub fn state(request: &str) -> String {
    respond(request, |req: StateRequest| {
        with_fluid(&req.fluid, |f| solve(f, &req.inputs))
    })
}

/// Request `{fluid, inputs: [{..}, ..]}` → `{"ok": [{"ok": State} |
/// {"error": ..}, ..]}`: one envelope per input set, so one bad point does
/// not fail the batch.
pub fn states(request: &str) -> String {
    respond(request, |req: StatesRequest| {
        with_fluid(&req.fluid, |f| {
            Ok(req
                .inputs
                .iter()
                .map(|inputs| Envelope::from(solve(f, inputs)))
                .collect::<Vec<_>>())
        })
    })
}

#[derive(Deserialize)]
struct FluidRequest {
    fluid: String,
}

#[derive(Deserialize)]
struct StateRequest {
    fluid: String,
    inputs: Map<String, Value>,
}

#[derive(Deserialize)]
struct StatesRequest {
    fluid: String,
    inputs: Vec<Map<String, Value>>,
}

/// Isoline resolution cap: a request cannot stall the page.
const MAX_POINTS: usize = 2000;

#[derive(Deserialize)]
struct DiagramRequest {
    fluid: String,
    diagram: String,
    #[serde(default)]
    isolines: Vec<IsolineSpec>,
    #[serde(default = "default_points")]
    points: usize,
    #[serde(default)]
    limits: Option<Limits>,
}

fn default_points() -> usize {
    100
}

#[derive(Deserialize)]
struct IsolineSpec {
    kind: String,
    /// Explicit values; otherwise `count` suggested ones (default 5).
    #[serde(default)]
    values: Option<Vec<f64>>,
    #[serde(default)]
    count: Option<usize>,
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

#[derive(Serialize)]
struct FluidInfo {
    data: &'static FluidData,
    critical: CriticalPoint,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Envelope<T> {
    Ok(T),
    Error(ErrorBody),
}

impl<T> From<Result<T, PropsError>> for Envelope<T> {
    fn from(result: Result<T, PropsError>) -> Self {
        match result {
            Ok(v) => Self::Ok(v),
            Err(e) => Self::Error(ErrorBody::from(e)),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    kind: &'static str,
    message: String,
}

impl From<PropsError> for ErrorBody {
    fn from(e: PropsError) -> Self {
        let kind = match e {
            PropsError::UnknownFluid(_) => "unknown_fluid",
            PropsError::InvalidInput(_) => "invalid_input",
            PropsError::CoolProp(_) => "coolprop",
        };
        Self {
            kind,
            message: e.to_string(),
        }
    }
}

fn ok<T: Serialize>(payload: T) -> String {
    to_json(&Envelope::<T>::Ok(payload))
}

/// Parses `request`, runs `f` and wraps the outcome in an envelope.
fn respond<Req, T>(request: &str, f: impl FnOnce(Req) -> Result<T, PropsError>) -> String
where
    Req: for<'de> Deserialize<'de>,
    T: Serialize,
{
    let result = serde_json::from_str(request)
        .map_err(|e| PropsError::InvalidInput(format!("malformed request: {e}")))
        .and_then(f);
    to_json(&Envelope::from(result))
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|e| {
        // Only non-finite floats could fail, and payloads never carry them.
        format!(r#"{{"error":{{"kind":"coolprop","message":"serialize failed: {e}"}}}}"#)
    })
}

thread_local! {
    /// One open `Fluid` per curated fluid, reused across calls: opening an
    /// `AbstractState` costs far more than solving a state with it. The
    /// catalog bounds the size; wasm runs on a single thread.
    static FLUIDS: RefCell<HashMap<Variant, Rc<Fluid>>> = RefCell::new(HashMap::new());
}

/// Runs `f` with the cached session for `name` (canonical name or alias),
/// opening it on first use.
fn with_fluid<T>(
    name: &str,
    f: impl FnOnce(&Fluid) -> Result<T, PropsError>,
) -> Result<T, PropsError> {
    let variant = Variant::from_name(name).ok_or_else(|| PropsError::UnknownFluid(name.into()))?;
    let cached = FLUIDS.with_borrow(|m| m.get(&variant).cloned());
    let fluid = match cached {
        Some(fluid) => fluid,
        None => {
            let fluid = Rc::new(Fluid::new(variant)?);
            FLUIDS.with_borrow_mut(|m| m.insert(variant, Rc::clone(&fluid)));
            fluid
        }
    };
    f(&fluid)
}

fn solve(fluid: &Fluid, inputs: &Map<String, Value>) -> Result<State, PropsError> {
    let mut parsed = inputs.iter().map(|(k, v)| parse_input(k, v));
    match (parsed.next(), parsed.next(), parsed.next()) {
        (Some(a), Some(b), None) => fluid.state(a?, b?),
        _ => Err(PropsError::InvalidInput(format!(
            "expected exactly two inputs, got {}",
            inputs.len()
        ))),
    }
}

fn parse_kind(name: &str) -> Result<InputKind, PropsError> {
    InputKind::from_name(name).ok_or_else(|| {
        let known: Vec<&str> = InputKind::ALL.iter().map(|k| k.name()).collect();
        PropsError::InvalidInput(format!(
            "unknown input `{name}`; expected one of {}",
            known.join(", ")
        ))
    })
}

fn parse_input(name: &str, value: &Value) -> Result<Input, PropsError> {
    let kind = parse_kind(name)?;
    let v = value
        .as_f64()
        .ok_or_else(|| PropsError::InvalidInput(format!("input `{name}` must be a number")))?;
    Ok(kind.with_value(v))
}
