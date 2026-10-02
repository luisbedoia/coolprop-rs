//! The JSON contract seen by JavaScript, exercised natively.

use serde_json::{Value, json};

fn call(f: fn(&str) -> String, request: Value) -> Value {
    serde_json::from_str(&f(&request.to_string())).unwrap()
}

#[track_caller]
fn ok(envelope: &Value) -> &Value {
    envelope
        .get("ok")
        .unwrap_or_else(|| panic!("expected ok: {envelope}"))
}

#[track_caller]
fn error_kind(envelope: &Value) -> &str {
    envelope["error"]["kind"]
        .as_str()
        .unwrap_or_else(|| panic!("expected error: {envelope}"))
}

#[test]
fn version_is_coolprop_8() {
    let v: Value = serde_json::from_str(&coolprop_wasm::version()).unwrap();
    assert!(ok(&v).as_str().unwrap().starts_with("8."), "{v}");
}

#[test]
fn catalog_lists_fluids_with_aliases() {
    let v: Value = serde_json::from_str(&coolprop_wasm::catalog()).unwrap();
    let water = ok(&v)
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "Water")
        .expect("Water is curated");
    assert!(water["aliases"].as_array().unwrap().contains(&json!("H2O")));
    for key in [
        "cas",
        "formula",
        "molar_mass",
        "acentric",
        "t_triple",
        "p_triple",
        "t_max",
        "p_max",
    ] {
        assert!(water.get(key).is_some(), "missing {key}");
    }
}

#[test]
fn fluid_reports_data_and_eos_critical_point() {
    let v = call(coolprop_wasm::fluid, json!({"fluid": "H2O"}));
    let info = ok(&v);
    assert_eq!(info["data"]["name"], "Water");
    let tc = info["critical"]["temperature"].as_f64().unwrap();
    assert!((tc - 647.096).abs() < 1e-3, "{tc}");
}

#[test]
fn state_from_named_inputs() {
    let v = call(
        coolprop_wasm::state,
        json!({"fluid": "Water", "inputs": {"temperature": 298.15, "pressure": 101325.0}}),
    );
    let s = ok(&v);
    assert_eq!(s["phase"], "liquid");
    assert!(s["quality"].is_null());
    let h = s["enthalpy"].as_f64().unwrap();
    assert!((h - 104_920.0).abs() < 1.0, "{h}");
}

#[test]
fn state_errors_are_classified() {
    let kind = |req| error_kind(&call(coolprop_wasm::state, req)).to_owned();
    assert_eq!(
        kind(json!({"fluid": "Unobtainium", "inputs": {}})),
        "unknown_fluid"
    );
    assert_eq!(
        kind(json!({"fluid": "Water", "inputs": {"pressure": 1e5}})),
        "invalid_input"
    );
    assert_eq!(
        kind(json!({"fluid": "Water", "inputs": {"pressure": 1e5, "colour": 3}})),
        "invalid_input"
    );
    assert_eq!(
        kind(json!({"fluid": "Water", "inputs": {"quality": 0.5, "internal_energy": 2e5}})),
        "invalid_input"
    );
    assert_eq!(
        kind(json!({"fluid": "Water", "inputs": {"pressure": 101325.0, "temperature": -10.0}})),
        "coolprop"
    );
    let v: Value = serde_json::from_str(&coolprop_wasm::state("{not json")).unwrap();
    assert_eq!(error_kind(&v), "invalid_input");
}

#[test]
fn states_batch_reports_each_point() {
    let v = call(
        coolprop_wasm::states,
        json!({"fluid": "Water", "inputs": [
            {"pressure": 101325.0, "quality": 0.0},
            {"pressure": 101325.0, "temperature": -10.0},
            {"pressure": 101325.0, "quality": 1.0},
        ]}),
    );
    let items = ok(&v).as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["ok"]["quality"], 0.0);
    assert_eq!(error_kind(&items[1]), "coolprop");
    assert_eq!(items[2]["ok"]["quality"], 1.0);
}
