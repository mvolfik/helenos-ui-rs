#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![doc = include_str!("../README.md")]

#[cfg(target_os = "helenos")]
pub use libc::{errno_t, sysarg_t};

#[cfg(target_os = "helenos")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
