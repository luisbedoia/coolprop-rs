//! The `schema` catalogs must describe the API exactly as it behaves.

use coolprop::plot::{Diagram, PlotProperty};
use coolprop::schema::{self, PropertyCategory};
use coolprop::{Fluid, Input, InputKind, Phase, PropsError, Variant};

fn water() -> Fluid {
    Fluid::new(Variant::Water).unwrap()
}

/// Keys of a serialized `State`, in field order.
fn state_keys(state: &coolprop::State) -> Vec<String> {
    match serde_json::to_value(state).unwrap() {
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        other => panic!("State serialized as {other}"),
    }
}

#[test]
fn properties_are_exactly_the_state_fields() {
    let s = water()
        .state(Input::Pressure(101_325.0), Input::Temperature(298.15))
        .unwrap();
    let mut catalog: Vec<&str> = schema::properties().iter().map(|p| p.name).collect();
    catalog.push("phase");
    let mut fields = state_keys(&s);
    let mut catalog_sorted = catalog.clone();
    fields.sort();
    catalog_sorted.sort();
    assert_eq!(catalog_sorted, fields);
}

#[test]
fn non_nullable_properties_are_always_present() {
    let f = water();
    for (a, b) in [
        (Input::Pressure(101_325.0), Input::Temperature(298.15)),
        (Input::Pressure(101_325.0), Input::Quality(0.5)),
        (Input::Pressure(30e6), Input::Temperature(700.0)),
    ] {
        let json = serde_json::to_value(f.state(a, b).unwrap()).unwrap();
        for p in schema::properties().iter().filter(|p| !p.nullable) {
            assert!(
                json[p.name].is_number(),
                "{} missing for ({a:?}, {b:?})",
                p.name
            );
        }
    }
}

#[test]
fn transport_category_holds_transport_properties() {
    let transport: Vec<&str> = schema::properties()
        .iter()
        .filter(|p| p.category == PropertyCategory::Transport)
        .map(|p| p.name)
        .collect();
    assert_eq!(transport, ["viscosity", "conductivity", "prandtl"]);
}

#[test]
fn inputs_cover_every_kind_in_order() {
    let names: Vec<InputKind> = schema::inputs().iter().map(|i| i.name).collect();
    assert_eq!(names, InputKind::ALL);
    for kind in InputKind::ALL {
        assert_eq!(InputKind::from_name(kind.name()), Some(kind));
        assert_eq!(kind.with_value(1.0).kind(), kind);
    }
    let quality = schema::inputs()
        .iter()
        .find(|i| i.name == InputKind::Quality)
        .unwrap();
    assert_eq!((quality.min, quality.max), (Some(0.0), Some(1.0)));
}

/// Every one of the 49 ordered combinations: listed pairs must be accepted
/// (in both orders), all others rejected as invalid input.
#[test]
fn pairs_match_what_state_accepts() {
    let f = water();
    // A two-phase reference state, so every pair (including quality) is
    // physically consistent.
    let r = f
        .state(Input::Pressure(101_325.0), Input::Quality(0.5))
        .unwrap();
    let value = |k: InputKind| match k {
        InputKind::Pressure => r.pressure(),
        InputKind::Temperature => r.temperature(),
        InputKind::Density => r.density(),
        InputKind::Enthalpy => r.enthalpy(),
        InputKind::Entropy => r.entropy(),
        InputKind::InternalEnergy => r.internal_energy(),
        InputKind::Quality => 0.5,
    };
    let listed: Vec<(InputKind, InputKind)> = schema::pairs().collect();
    assert_eq!(listed.len(), 14);
    for a in InputKind::ALL {
        for b in InputKind::ALL {
            let supported = listed.contains(&(a, b)) || listed.contains(&(b, a));
            let result = f.state(a.with_value(value(a)), b.with_value(value(b)));
            match (supported, result) {
                (true, Err(PropsError::InvalidInput(e))) => {
                    panic!("({a:?}, {b:?}) is listed but rejected: {e}")
                }
                (false, Err(PropsError::InvalidInput(_))) => {}
                (false, other) => panic!("({a:?}, {b:?}) is not listed but gave {other:?}"),
                // Listed pairs may still fail numerically (CoolProp error);
                // what matters is that the pair itself is accepted.
                (true, _) => {}
            }
        }
    }
}

#[test]
fn phases_cover_every_phase() {
    let names: Vec<Phase> = schema::phases().map(|p| p.name).collect();
    assert_eq!(names, Phase::ALL);
    assert!(schema::phases().all(|p| !p.label.is_empty() && !p.description.is_empty()));
}

#[test]
fn plot_properties_cover_every_axis_property_in_order() {
    let names: Vec<PlotProperty> = schema::plot_properties().iter().map(|p| p.name).collect();
    assert_eq!(names, PlotProperty::ALL);
}

#[test]
fn diagram_ids_are_unique_and_resolve_back() {
    let diagrams = schema::diagrams();
    assert_eq!(diagrams.len(), Diagram::all().len());
    let mut ids: Vec<&str> = diagrams.iter().map(|d| d.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), diagrams.len(), "ids are unique");
    for info in &diagrams {
        let d = Diagram::from_id(&info.id).expect("id resolves");
        assert_eq!((d.x, d.y), (info.x, info.y));
        assert_eq!(d.isoline_kinds(), info.isolines);
    }
    assert!(ids.contains(&"pressure_enthalpy") && ids.contains(&"temperature_entropy"));
}
