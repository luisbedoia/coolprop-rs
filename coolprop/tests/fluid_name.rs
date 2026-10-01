use coolprop::Fluid;
use coolprop_sys::catalog::Variant;

#[test]
fn variant_lookup_accepts_lowercase_names() {
    assert_eq!(Variant::from_name("water"), Some(Variant::Water));
    assert_eq!(Variant::from_name("WATER"), Some(Variant::Water));
}

#[test]
fn fluid_from_name_opens_abstract_state() {
    let fluid = Fluid::from_name("water").expect("water must resolve");
    let state = fluid
        .state()
        .expect("water state should open");

    assert!(state.temperature().abs() > 0.0);
}
