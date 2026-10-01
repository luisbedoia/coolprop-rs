//! Configure and run CMake to build CoolProp as a static library, then locate
//! the resulting archive and emit the `rustc-link-*` directives that hook it
//! into the parent build.

use std::env;
use std::path::{Path, PathBuf};

use crate::BuildEnv;

pub(crate) fn build_and_link(env: &BuildEnv) {
    let dst = run_cmake(env);
    let lib_path = locate_static_lib(&dst, &env.install_root, env.is_emscripten);
    emit_link_directives(lib_path.parent().unwrap(), env.is_emscripten);
}

fn run_cmake(env: &BuildEnv) -> PathBuf {
    let mut cfg = cmake::Config::new(&env.manifest_dir);
    cfg.define("COOLPROP_STATIC_LIBRARY", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("COOLPROP_RELEASE", "ON")
        .define(
            "COOLPROP_INSTALL_PREFIX",
            env.install_root.to_string_lossy().as_ref(),
        )
        .define(
            "OVERRIDES_DIR",
            env.overrides_dir.to_string_lossy().as_ref(),
        )
        .define("CPM_SOURCE_CACHE", cpm_source_cache(env))
        .cxxflag("-DCOOLPROP_LIB")
        .cxxflag("-DCOOLPROP_NO_INCBIN")
        .profile("Release");

    if env.is_emscripten {
        apply_emscripten_config(&mut cfg);
    }
    cfg.build()
}

/// Where CPM keeps the downloaded C++ dependencies (~550 MB: Eigen, fmt,
/// boost headers, ...). Upstream intends `CoolProp/.cpm_cache` but its default
/// never applies (CPM.cmake is included first and caches `OFF`), which makes
/// every OUT_DIR download its own copy. `CPM_SOURCE_CACHE` env var wins.
fn cpm_source_cache(env: &BuildEnv) -> String {
    env::var("CPM_SOURCE_CACHE").unwrap_or_else(|_| {
        env.manifest_dir
            .join("CoolProp/.cpm_cache")
            .to_string_lossy()
            .into_owned()
    })
}

fn apply_emscripten_config(cfg: &mut cmake::Config) {
    let emsdk = env::var("EMSDK")
        .unwrap_or_else(|_| panic!("EMSDK is not set. Source emsdk_env.sh before building."));
    let toolchain = PathBuf::from(&emsdk)
        .join("upstream")
        .join("emscripten")
        .join("cmake")
        .join("Modules")
        .join("Platform")
        .join("Emscripten.cmake");
    cfg.define("CMAKE_TOOLCHAIN_FILE", toolchain.to_string_lossy().as_ref());

    cfg.define(
        "CMAKE_CXX_FLAGS_RELEASE",
        "-O3 -DNDEBUG -fexceptions -fwasm-exceptions",
    )
    .define(
        "CMAKE_C_FLAGS_RELEASE",
        "-O3 -DNDEBUG -fexceptions -fwasm-exceptions",
    );
}

fn locate_static_lib(dst: &Path, install_root: &Path, is_emscripten: bool) -> PathBuf {
    let libname = if is_emscripten || !cfg!(target_os = "windows") {
        "libCoolProp.a"
    } else {
        "CoolProp.lib"
    };
    find_file(dst, libname)
        .or_else(|| find_file(install_root, libname))
        .unwrap_or_else(|| {
            panic!(
                "{} not produced by CMake build under {} or {}",
                libname,
                dst.display(),
                install_root.display()
            )
        })
}

fn emit_link_directives(lib_dir: &Path, is_emscripten: bool) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=CoolProp");

    // Emscripten's C++ runtime comes from `-sDEFAULT_TO_CXX` at the final
    // link (see .cargo/config.toml); a build script cannot propagate it.
    if !is_emscripten {
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-lib=dylib=c++");
        } else if cfg!(target_os = "linux") {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
    }
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(hit) = find_file(&path, name) {
                return Some(hit);
            }
        } else if path.file_name().and_then(|s| s.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}
