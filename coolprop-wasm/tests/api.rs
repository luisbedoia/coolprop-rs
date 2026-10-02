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

#[test]
fn schema_describes_inputs_pairs_properties_and_phases() {
    let v: Value = serde_json::from_str(&coolprop_wasm::schema()).unwrap();
    let schema = ok(&v);
    let quality = schema["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "quality")
        .unwrap();
    assert_eq!(
        quality,
        &json!({"name": "quality", "symbol": "x", "unit": "",
        "description": "Vapor quality (vapor mass fraction)", "min": 0.0, "max": 1.0})
    );
    assert!(
        schema["pairs"]
            .as_array()
            .unwrap()
            .contains(&json!(["pressure", "temperature"]))
    );
    let cp = schema["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "cp")
        .unwrap();
    assert_eq!(cp["unit"], "J/(kg·K)");
    assert_eq!(cp["nullable"], true);
    assert_eq!(cp["category"], "thermodynamic");
    assert_eq!(
        schema["phases"][6],
        json!({"name": "two_phase", "label": "two-phase",
        "description": "Liquid-vapor mixture inside the saturation dome"})
    );
}

#[test]
fn schema_describes_the_diagram_catalog() {
    let v: Value = serde_json::from_str(&coolprop_wasm::schema()).unwrap();
    let schema = ok(&v);
    assert_eq!(schema["plot_properties"].as_array().unwrap().len(), 7);
    let diagrams = schema["diagrams"].as_array().unwrap();
    assert_eq!(diagrams.len(), 6);
    let ph = diagrams
        .iter()
        .find(|d| d["id"] == "pressure_enthalpy")
        .unwrap();
    assert_eq!(ph["x"], json!({"property": "enthalpy", "scale": "linear"}));
    assert_eq!(ph["y"], json!({"property": "pressure", "scale": "log"}));
    let pt = diagrams
        .iter()
        .find(|d| d["id"] == "pressure_temperature")
        .unwrap();
    assert_eq!(pt["y"], json!({"property": "pressure", "scale": "linear"}));
    let isolines = ph["isolines"].as_array().unwrap();
    assert!(isolines.contains(&json!("temperature")) && !isolines.contains(&json!("pressure")));
}

#[test]
fn diagram_projects_dome_and_isolines_onto_its_axes() {
    let v = call(
        coolprop_wasm::diagram,
        json!({"fluid": "Water", "diagram": "pressure_enthalpy", "points": 40, "isolines": [
            {"kind": "temperature", "count": 3},
            {"kind": "quality", "values": [0.5]},
        ]}),
    );
    let d = ok(&v);
    assert_eq!(d["x"]["property"], "enthalpy");
    assert_eq!(d["y"]["scale"], "log");
    let range = d["y"]["range"].as_array().unwrap();
    assert!(range[0].as_f64().unwrap() < range[1].as_f64().unwrap());

    let liquid = &d["dome"]["liquid"];
    assert_eq!(
        liquid["x"].as_array().unwrap().len(),
        liquid["y"].as_array().unwrap().len()
    );

    let isolines = d["isolines"].as_array().unwrap();
    assert_eq!(isolines.len(), 4, "3 isotherms + 1 iso-quality");
    assert_eq!(isolines[3]["kind"], "quality");
    assert_eq!(isolines[3]["value"], 0.5);
    // Every point of the x = 0.5 line sits on the dome: enthalpy between
    // the branches, pressure on the saturation curve.
    for p in isolines[3]["y"].as_array().unwrap() {
        let p = p.as_f64().expect("iso-quality is fully solved");
        assert!(p > 0.0);
    }
}

#[test]
fn suggested_values_are_round_in_the_requested_unit() {
    // Isotherms in °C on h–s: an even grid of round values.
    let v = call(
        coolprop_wasm::diagram,
        json!({"fluid": "Water", "diagram": "enthalpy_entropy", "points": 10, "isolines": [
            {"kind": "temperature", "count": 7, "unit": {"scale": 1.0, "offset": -273.15}},
        ]}),
    );
    let celsius: Vec<f64> = ok(&v)["isolines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|iso| iso["value"].as_f64().unwrap() - 273.15)
        .collect();
    let expected = [50.0, 100.0, 150.0, 200.0, 250.0, 300.0, 350.0];
    for (c, e) in celsius.iter().zip(expected) {
        assert!((c - e).abs() < 1e-9, "{celsius:?}");
    }
}

#[test]
fn unsolvable_points_are_null() {
    // This isentrope leaves the domain partway: breaks are null, the rest
    // is data.
    let v = call(
        coolprop_wasm::diagram,
        json!({"fluid": "Water", "diagram": "pressure_enthalpy", "points": 40,
               "isolines": [{"kind": "entropy", "values": [12668.0]}]}),
    );
    let xs = ok(&v)["isolines"][0]["x"].as_array().unwrap().clone();
    assert!(xs.iter().any(Value::is_null), "breaks are null");
    assert!(xs.iter().any(Value::is_number), "and the rest is data");
}

#[test]
fn diagram_errors_are_classified() {
    let kind = |req| error_kind(&call(coolprop_wasm::diagram, req)).to_owned();
    assert_eq!(
        kind(json!({"fluid": "Nope", "diagram": "pressure_enthalpy"})),
        "unknown_fluid"
    );
    assert_eq!(
        kind(json!({"fluid": "Water", "diagram": "nope"})),
        "invalid_input"
    );
    assert_eq!(
        kind(json!({"fluid": "Water", "diagram": "pressure_enthalpy",
                    "isolines": [{"kind": "pressure"}]})),
        "invalid_input",
        "an axis is not an isoline family"
    );
    assert_eq!(
        kind(json!({"fluid": "Water", "diagram": "pressure_enthalpy", "points": 1_000_000})),
        "invalid_input"
    );
}
