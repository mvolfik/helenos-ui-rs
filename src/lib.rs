#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![doc = include_str!("../README.md")]

pub use libc::{errno_t, sysarg_t};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
