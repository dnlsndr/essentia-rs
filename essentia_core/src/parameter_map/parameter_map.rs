//! [`ParameterMap`] — staging area for parameter values before configure.
//!
//! Essentia's algorithms expect their parameters to be supplied as a single
//! `ParameterMap` at configure time, not one parameter at a time. This
//! struct is the Rust-side accumulator: each `set_parameter` call sticks
//! one value in, and `Algorithm::configure` hands the whole map across the
//! FFI boundary in one shot.
//!
//! Users normally don't construct a `ParameterMap` directly — it lives
//! inside [`Algorithm<Initialized>`](crate::algorithm::Algorithm) and is
//! managed by its parameter-setter methods.

use cxx::UniquePtr;
use essentia_sys::ffi;

use crate::data::DataContainer;

/// Accumulator for parameter values destined for an
/// [`Algorithm`](crate::Algorithm)'s configure call.
///
/// Wraps an FFI [`ParameterMapBridge`](essentia_sys::ffi::ParameterMapBridge)
/// — itself a thin shim over Essentia's C++ `ParameterMap`.
pub struct ParameterMap {
    /// Owned C++ parameter map. Crate-private so that
    /// [`Algorithm::configure`](crate::algorithm::Algorithm) can move it
    /// across the FFI boundary without exposing the cxx detail more
    /// broadly.
    pub(crate) parameter_map_bridge: UniquePtr<ffi::ParameterMapBridge>,
}

impl Default for ParameterMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ParameterMap {
    /// Construct a fresh, empty parameter map. Allocates a new
    /// [`ParameterMapBridge`](essentia_sys::ffi::ParameterMapBridge) on the
    /// C++ side.
    pub fn new() -> Self {
        Self {
            parameter_map_bridge: ffi::create_parameter_map_bridge(),
        }
    }

    /// Insert a parameter into the map.
    ///
    /// `value` is consumed (its underlying C++ object is moved into the
    /// map). The static type marker `T` only constrains what callers can
    /// pass; the FFI handles every type uniformly.
    pub fn set_parameter<T>(&mut self, key: &str, value: DataContainer<'static, T>) {
        // The C++ side returns `Result<()>` only because cxx requires it;
        // in practice this call cannot fail for any reason callers would
        // need to handle, hence the unwrap.
        self.parameter_map_bridge
            .pin_mut()
            .add(key, value.into_owned_ptr())
            .unwrap();
    }
}
