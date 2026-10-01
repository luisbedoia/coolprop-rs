//! Generate Rust FFI bindings from `wrapper.h` via bindgen.

use crate::BuildEnv;

pub(crate) fn generate(env: &BuildEnv) {
    let wrapper = env.manifest_dir.join("wrapper.h");
    println!("cargo:rerun-if-changed={}", wrapper.display());

    let mut builder = bindgen::Builder::default()
        .header(wrapper.to_string_lossy())
        .allowlist_file(".*CoolPropLib\\.h")
        .rust_edition(bindgen::RustEdition::Edition2024)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    if env.is_emscripten {
        builder = builder.clang_arg("--target=x86_64-unknown-linux-gnu");
    }
    let bindings = builder
        .generate()
        .expect("Unable to generate CoolProp bindings");

    bindings
        .write_to_file(env.out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings.rs");
}
