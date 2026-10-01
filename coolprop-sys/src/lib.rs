//! Raw FFI bindings to the CoolProp C++ library.
//!
//! CoolProp is built from the `CoolProp/` git submodule and linked
//! statically. Only the fluids in the curated catalog ([`catalog`]) are
//! embedded in the library.

pub mod catalog;

#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    unreachable_pub,
    clippy::all
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;
