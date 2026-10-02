#[test]
fn version_is_nonempty() {
    let v = coolprop::version();
    assert!(!v.is_empty(), "version() returned empty string");
    println!("CoolProp version: {}", v);
}
