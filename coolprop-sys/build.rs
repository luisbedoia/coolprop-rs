// Placeholder: the CMake build of the CoolProp submodule, the curated fluid
// catalog and the bindgen step are ported in the next phase.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
