//! JSON API behind the wasm module, kept free of FFI so it builds and is
//! tested natively. `main.rs` only exposes it as `extern "C"` functions.
//!
//! Every function returns a JSON envelope: `{"ok": <payload>}` or
//! `{"error": {"kind": "unknown_fluid" | "invalid_input" | "coolprop",
//! "message": "..."}}`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use coolprop::{CriticalPoint, Fluid, FluidData, Input, PropsError, State, Variant};
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

fn parse_input(name: &str, value: &Value) -> Result<Input, PropsError> {
    let v = value
        .as_f64()
        .ok_or_else(|| PropsError::InvalidInput(format!("input `{name}` must be a number")))?;
    Ok(match name {
        "pressure" => Input::Pressure(v),
        "temperature" => Input::Temperature(v),
        "density" => Input::Density(v),
        "enthalpy" => Input::Enthalpy(v),
        "entropy" => Input::Entropy(v),
        "internal_energy" => Input::InternalEnergy(v),
        "quality" => Input::Quality(v),
        other => {
            return Err(PropsError::InvalidInput(format!(
                "unknown input `{other}`; expected one of pressure, temperature, \
                 density, enthalpy, entropy, internal_energy, quality"
            )));
        }
    })
}
