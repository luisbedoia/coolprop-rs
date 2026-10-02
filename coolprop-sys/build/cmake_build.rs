//! Configure and run CMake to build CoolProp as a static library, then locate
//! the resulting archive and emit the `rustc-link-*` directives that hook it
//! into the parent build.

use std::env;
use std::path::{Path, PathBuf};

use crate::BuildEnv;

pub(crate) fn build_and_link(env: &BuildEnv) {
    let dst = run_cmake(env);
    let lib_path = locate_static_lib(&dst, env);
    emit_link_directives(lib_path.parent().unwrap(), env);
}

fn run_cmake(env: &BuildEnv) -> PathBuf {
    let mut cfg = cmake::Config::new(&env.manifest_dir);
    cfg.define("COOLPROP_STATIC_LIBRARY", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("COOLPROP_RELEASE", "ON")
        // Otherwise CoolProp adds -m64/-m32 by pointer size, which GCC on
        // non-x86 targets (aarch64 Linux) rejects; the target decides.
        .define("FORCE_BITNESS_NATIVE", "ON")
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

    // Optimization flags for CoolProp's C++, overridable with
    // COOLPROP_WASM_OPT (e.g. "-Os -flto") to trade speed for size.
    let opt = env::var("COOLPROP_WASM_OPT").unwrap_or_else(|_| "-O3".to_owned());
    let flags = format!("{opt} -DNDEBUG -fexceptions -fwasm-exceptions");
    cfg.define("CMAKE_CXX_FLAGS_RELEASE", &flags)
        .define("CMAKE_C_FLAGS_RELEASE", &flags);
}

/// The CoolProp archive this build produced. The build tree keeps archives
/// from earlier builds (CoolProp installs into a directory named after the
/// compiler and bitness), so pick the most recently written one rather than
/// whichever a directory walk happens to reach first.
fn locate_static_lib(dst: &Path, env: &BuildEnv) -> PathBuf {
    // Only MSVC names static libraries `*.lib`; MinGW and Unix use `lib*.a`.
    let libname = if env.target_env == "msvc" {
        "CoolProp.lib"
    } else {
        "libCoolProp.a"
    };
    let mut found = Vec::new();
    find_files(dst, libname, &mut found);
    if !env.install_root.starts_with(dst) {
        find_files(&env.install_root, libname, &mut found);
    }
    found
        .into_iter()
        .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok())
        .unwrap_or_else(|| {
            panic!(
                "{libname} not produced by CMake build under {} or {}",
                dst.display(),
                env.install_root.display()
            )
        })
}

fn emit_link_directives(lib_dir: &Path, env: &BuildEnv) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=CoolProp");

    // Emscripten's C++ runtime comes from `-sDEFAULT_TO_CXX` at the final
    // link (see .cargo/config.toml); a build script cannot propagate it.
    // Otherwise link the target's C++ standard library (MSVC links its own).
    if !env.is_emscripten {
        match env.target_os.as_str() {
            "macos" | "ios" | "freebsd" => println!("cargo:rustc-link-lib=dylib=c++"),
            "linux" => println!("cargo:rustc-link-lib=dylib=stdc++"),
            _ => {}
        }
    }
}

/// Every file named `name` under `root`, recursively.
fn find_files(root: &Path, name: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_files(&path, name, found);
        } else if path.file_name().and_then(|s| s.to_str()) == Some(name) {
            found.push(path);
        }
    }
}
