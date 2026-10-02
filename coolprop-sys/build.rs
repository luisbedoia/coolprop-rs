use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "build/fluids.rs"]
mod fluids;

#[path = "build/catalog.rs"]
mod catalog;

#[path = "build/cbor.rs"]
mod cbor;

#[path = "build/overrides.rs"]
mod overrides;

#[path = "build/cmake_build.rs"]
mod cmake_build;

#[path = "build/bindings.rs"]
mod bindings;

pub(crate) struct BuildEnv {
    pub(crate) is_emscripten: bool,
    /// Target OS and environment (`CARGO_CFG_TARGET_*`). Not `cfg!`, which
    /// in a build script describes the host it runs on.
    pub(crate) target_os: String,
    pub(crate) target_env: String,
    pub(crate) manifest_dir: PathBuf,
    pub(crate) out_dir: PathBuf,
    pub(crate) install_root: PathBuf,
    pub(crate) overrides_dir: PathBuf,
    /// Fluid stems to bake in: `COOLPROP_FLUIDS` env override, else
    /// [`fluids::CURATED_FLUIDS`].
    pub(crate) fluids: Vec<String>,
}

impl BuildEnv {
    fn from_cargo() -> Self {
        let target = env::var("TARGET").expect("TARGET not set by cargo");
        let is_emscripten = target.contains("emscripten");
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let install_root = out_dir.join("coolprop-install");
        let overrides_dir = out_dir.join("include_overrides");
        let fluids = fluids::resolve_fluids();
        Self {
            is_emscripten,
            target_os,
            target_env,
            manifest_dir,
            out_dir,
            install_root,
            overrides_dir,
            fluids,
        }
    }

    fn ensure_submodule_present(&self) {
        let coolprop_src = self.manifest_dir.join("CoolProp");
        if !coolprop_src.exists() {
            panic!(
                "CoolProp source not found at {}. \
                 Did you run `git submodule update --init --recursive`?",
                coolprop_src.display()
            );
        }
    }

    fn recreate_overrides_dir(&self) {
        if self.overrides_dir.exists() {
            fs::remove_dir_all(&self.overrides_dir).expect("clear include_overrides");
        }
        fs::create_dir_all(&self.overrides_dir).expect("recreate include_overrides");
    }

    fn emit_rerun_directives(&self) {
        let fluids_dir = self.manifest_dir.join("CoolProp/dev/fluids");
        let mixtures_dir = self.manifest_dir.join("CoolProp/dev/mixtures");
        println!("cargo:rerun-if-changed={}", fluids_dir.display());
        println!("cargo:rerun-if-changed={}", mixtures_dir.display());
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=build");
        println!("cargo:rerun-if-changed=CMakeLists.txt");
        println!("cargo:rerun-if-env-changed=COOLPROP_FLUIDS");
        println!("cargo:rerun-if-env-changed=CPM_SOURCE_CACHE");
        println!("cargo:rerun-if-env-changed=COOLPROP_WASM_OPT");
    }
}

fn main() {
    let env = BuildEnv::from_cargo();
    env.ensure_submodule_present();
    env.recreate_overrides_dir();

    catalog::generate_rust_catalog(&env);
    overrides::generate_c_overrides(&env);

    cmake_build::build_and_link(&env);
    bindings::generate(&env);

    env.emit_rerun_directives();
}
