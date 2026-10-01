# Wrapper CMakeLists used by `coolprop-sys/build.rs`.
#
# Purpose: build CoolProp from the unmodified `CoolProp/` git submodule, but
# with our `include_overrides/` directory prepended to the include search
# path so we can shadow specific upstream headers (notably the bulky
# `all_fluids_CBOR.h` that embeds every fluid's definition; build.rs
# generates a slim replacement containing only the curated fluids).
#
# The upstream CMakeLists.txt is consumed verbatim via `add_subdirectory`.
# It uses `CMAKE_CURRENT_SOURCE_DIR` everywhere, so its globs and include
# paths resolve correctly relative to its own location.

cmake_minimum_required(VERSION 3.14)
project(coolprop-sys-wrapper CXX C)

# `OVERRIDES_DIR` is set by build.rs to the absolute path of the generated
# override headers under OUT_DIR. `BEFORE` puts it ahead of every later
# `include_directories(...)` call (including CoolProp's), so when the
# preprocessor resolves `#include "all_fluids_CBOR.h"` from
# FluidLibrary.cpp it finds our slim copy first.
if(DEFINED OVERRIDES_DIR)
    include_directories(BEFORE "${OVERRIDES_DIR}")
endif()

add_subdirectory(CoolProp)
