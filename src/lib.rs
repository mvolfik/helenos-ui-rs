#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![doc = include_str!("../README.md")]

#[cfg(target_os = "helenos")]
pub use libc::{errno_t, sysarg_t};

#[cfg(target_os = "helenos")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub mod util {
    #[cfg(target_os = "helenos")]
    use super::errno_t;
    #[cfg(not(target_os = "helenos"))]
    type errno_t = i32;

    use std::convert::Infallible;

    pub trait IntoError {
        type Error: std::error::Error;
        fn into_error(self) -> Result<(), Self::Error>;
    }

    impl IntoError for errno_t {
        type Error = std::io::Error;

        fn into_error(self) -> Result<(), std::io::Error> {
            if self == 0 {
                Ok(())
            } else {
                Err(std::io::Error::from_raw_os_error(self))
            }
        }
    }

    impl IntoError for () {
        type Error = Infallible;

        fn into_error(self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    pub fn pointer_init<T, E: IntoError, F: FnOnce(*mut T) -> E>(fun: F) -> Result<T, E::Error> {
        let mut uninit = std::mem::MaybeUninit::uninit();
        fun(uninit.as_mut_ptr()).into_error()?;
        Ok(unsafe { uninit.assume_init() })
    }
}
