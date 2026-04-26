//! [`Algorithm`] — the generic, typestate-driven wrapper around a single
//! Essentia algorithm.
//!
//! In Essentia, every algorithm follows the same lifecycle:
//!
//! 1. *Construct* the algorithm by name from the global factory.
//! 2. *Set parameters* (configuration knobs).
//! 3. *Configure* — Essentia validates and finalises the parameters,
//!    allocating internal state.
//! 4. *Set inputs and compute* — possibly many times in a row.
//! 5. *(optional)* *Reset* between computations to clear running state.
//!
//! This file expresses that lifecycle as a typestate machine: an
//! [`Algorithm<Initialized>`] only exposes parameter setters and a
//! [`configure`](Algorithm::configure) method; once configured, you only
//! get [`Algorithm<Configured>`]'s `set_input` / `compute` / `reset`. That
//! prevents whole classes of misuse at compile time.
//!
//! Note that this is the *generic* algorithm — it doesn't know which
//! Essentia algorithm it wraps. The per-algorithm builder structs in the
//! `essentia` crate are thin, statically-typed shells around this struct,
//! generated at build time from Essentia's introspection metadata.

use cxx::UniquePtr;
use essentia_sys::ffi;
use std::marker::PhantomData;

use crate::{
    IntoDataContainer,
    algorithm::{
        ComputeError, ConfigurationError, InputError, Introspection, OutputError, ParameterError,
        ResetError,
    },
    data::{DataContainer, InputOutputData, ParameterData, types::HasDataType},
    essentia::Essentia,
    parameter_map::ParameterMap,
};

/// Typestate marker: the algorithm has been created but not yet configured.
///
/// In this state only parameter setters and [`configure`](Algorithm::configure)
/// are available. The struct also stashes the [`ParameterMap`] of pending
/// parameter values until configure consumes it.
pub struct Initialized {
    /// Parameter values queued up for the upcoming `configure` call.
    parameter_map: ParameterMap,
}

/// Typestate marker: the algorithm has been configured. Inputs can now be
/// set and `compute` can be called.
///
/// Carries no payload — once configured, all the runtime state lives inside
/// the C++ side.
pub struct Configured;

/// A live Essentia algorithm in some lifecycle state.
///
/// `'a` is the lifetime of the [`Essentia`] handle that produced this
/// algorithm. Tying the algorithm to that lifetime prevents the global C++
/// runtime from being torn down while the algorithm is still alive.
///
/// `State` is the typestate marker — either [`Initialized`] or
/// [`Configured`] — and dictates which methods are available.
pub struct Algorithm<'a, State = Initialized> {
    /// The actual C++ algorithm bridge. The bridge is the FFI handle to a
    /// concrete `essentia::Algorithm` instance on the C++ side.
    algorithm_bridge: UniquePtr<ffi::AlgorithmBridge>,
    /// Typestate-specific data (e.g. the pending parameter map for
    /// `Initialized`). [`Configured`] is empty.
    state: State,
    /// Cached metadata about this algorithm's parameters and inputs/outputs,
    /// used to validate user-supplied keys without round-tripping through
    /// the FFI on every call.
    introspection: Introspection,
    /// Phantom borrow that ties the algorithm to the [`Essentia`] runtime
    /// handle that produced it.
    _marker: PhantomData<&'a Essentia>,
}

impl<'a, State> Algorithm<'a, State> {
    /// Read-only access to the introspection metadata of this algorithm.
    ///
    /// Available in any state — useful for inspecting the names, types,
    /// descriptions and constraints of an algorithm's parameters and
    /// inputs/outputs at runtime.
    pub fn introspection(&self) -> &Introspection {
        &self.introspection
    }
}

impl<'a> Algorithm<'a, Initialized> {
    /// Wrap a freshly-created C++ algorithm bridge.
    ///
    /// Crate-private — user code goes through
    /// [`Essentia::create_algorithm`](crate::Essentia::create_algorithm).
    pub(crate) fn new(algorithm_bridge: UniquePtr<ffi::AlgorithmBridge>) -> Self {
        // Pull introspection up front so that validation in `set_parameter`
        // is a HashMap lookup rather than a fresh FFI call each time.
        let introspection = Introspection::from_algorithm_bridge(&algorithm_bridge);

        Self {
            algorithm_bridge,
            state: Initialized {
                parameter_map: ParameterMap::new(),
            },
            introspection,
            _marker: PhantomData,
        }
    }

    /// Builder-style parameter setter: takes `self` by value so it can be
    /// chained.
    ///
    /// `key` is the parameter name as Essentia knows it (e.g.
    /// `"sampleRate"`). `value` is anything that can be turned into a
    /// [`DataContainer`] of the parameter's static type `T`.
    ///
    /// Internally just defers to [`Self::set_parameter`].
    pub fn parameter<T>(
        mut self,
        key: &str,
        value: impl IntoDataContainer<T>,
    ) -> Result<Self, ParameterError>
    where
        T: ParameterData + HasDataType,
    {
        self.set_parameter(key, value)?;
        Ok(self)
    }

    /// In-place parameter setter.
    ///
    /// Validates the parameter against introspection in two ways:
    ///
    /// 1. The parameter must exist on this algorithm — otherwise returns
    ///    [`ParameterError::ParameterNotFound`].
    /// 2. The parameter's expected [`DataType`](crate::DataType) must match
    ///    `T::data_type()` — otherwise returns
    ///    [`ParameterError::TypeMismatch`].
    ///
    /// On success, the value is *not* yet sent to C++; it is staged in the
    /// internal [`ParameterMap`] and shipped over in bulk on
    /// [`configure`](Self::configure).
    pub fn set_parameter<T>(
        &mut self,
        key: &str,
        value: impl IntoDataContainer<T>,
    ) -> Result<(), ParameterError>
    where
        T: ParameterData + HasDataType,
    {
        let param_info = self.introspection.get_parameter(key).ok_or_else(|| {
            ParameterError::ParameterNotFound {
                parameter: key.to_string(),
            }
        })?;

        let expected_type = T::data_type();
        let param_data_type = param_info.parameter_type();

        if param_data_type != expected_type {
            return Err(ParameterError::TypeMismatch {
                parameter: key.to_string(),
                expected: expected_type,
                actual: param_data_type,
            });
        }

        let data_container = value.into_data_container();

        self.state.parameter_map.set_parameter(key, data_container);

        Ok(())
    }

    /// Hand the staged parameters to C++ Essentia, transitioning the
    /// algorithm into the [`Configured`] state.
    ///
    /// This is where Essentia validates the parameter values against its
    /// own constraints (numeric ranges, mutually-exclusive options, file
    /// existence, etc.). Failures surface as [`ConfigurationError`].
    ///
    /// The transition consumes `self`, so the resulting `Configured`
    /// algorithm cannot accidentally be reverted to `Initialized`.
    pub fn configure(mut self) -> Result<Algorithm<'a, Configured>, ConfigurationError> {
        self.algorithm_bridge
            .pin_mut()
            .configure(self.state.parameter_map.parameter_map_bridge)?;

        Ok(Algorithm {
            algorithm_bridge: self.algorithm_bridge,
            state: Configured,
            introspection: self.introspection,
            _marker: PhantomData,
        })
    }
}

impl<'a> Algorithm<'a, Configured> {
    /// Builder-style input setter, analogous to
    /// [`parameter`](Algorithm::<Initialized>::parameter).
    ///
    /// In practice the auto-generated builders pass inputs through
    /// [`compute`](Algorithm::<Configured>::compute) instead — but this
    /// method exists for code that drives the generic algorithm directly.
    pub fn input<T>(
        mut self,
        key: &str,
        value: impl IntoDataContainer<T>,
    ) -> Result<Self, InputError>
    where
        T: InputOutputData + HasDataType,
    {
        self.set_input(key, value)?;
        Ok(self)
    }

    /// In-place input setter.
    ///
    /// Validates the input against introspection in the same two ways as
    /// [`set_parameter`](Algorithm::<Initialized>::set_parameter):
    /// the input must exist on this algorithm, and the static type `T`
    /// must match the input's declared type.
    ///
    /// Inputs *are* sent to C++ immediately (unlike parameters, which are
    /// staged until configure).
    pub fn set_input<T>(
        &mut self,
        key: &str,
        value: impl IntoDataContainer<T>,
    ) -> Result<(), InputError>
    where
        T: InputOutputData + HasDataType,
    {
        let input_info =
            self.introspection
                .get_input(key)
                .ok_or_else(|| InputError::InputNotFound {
                    input: key.to_string(),
                })?;

        let expected_type = T::data_type();
        let input_data_type = input_info.input_output_type();

        if input_data_type != expected_type {
            return Err(InputError::TypeMismatch {
                input: key.to_string(),
                expected: expected_type,
                actual: input_data_type,
            });
        }

        let data_container = value.into_data_container();

        let owned_ptr = data_container.into_owned_ptr();

        // The introspection check above guarantees this call cannot fail
        // for "input not found" reasons; any error here would be a bug.
        self.algorithm_bridge
            .pin_mut()
            .set_input(key, owned_ptr)
            .expect(&format!("failed to set input '{}' after validation", key));

        Ok(())
    }

    /// Run the algorithm's compute step against the inputs that have been
    /// set so far.
    ///
    /// Before invoking the C++ side, this iterates the algorithm's outputs
    /// (as known from introspection) and tells C++ Essentia which Rust-side
    /// type to materialise each output as. That two-step is required because
    /// Essentia's outputs need to be wired to a destination buffer before
    /// the algorithm runs.
    ///
    /// The returned [`ComputeResult`] borrows from `self`, which keeps the
    /// algorithm — and therefore its output buffers — alive for as long as
    /// the result is read.
    pub fn compute(&mut self) -> Result<ComputeResult<'a, '_>, ComputeError> {
        for output in self.introspection.outputs() {
            let data_type = output.input_output_type();

            self.algorithm_bridge
                .pin_mut()
                .setup_output(output.name(), data_type.into())
                .expect(&format!(
                    "failed to setup output '{}' after validation",
                    &output.name()
                ));
        }

        self.algorithm_bridge
            .pin_mut()
            .compute()
            .map_err(ComputeError::Compute)?;

        Ok(ComputeResult { algorithm: self })
    }

    /// Discard any state accumulated across previous compute calls.
    ///
    /// Some algorithms (running statistics, FFT buffers, …) carry state
    /// from one compute to the next. Calling `reset` returns them to their
    /// just-configured state without rebuilding the algorithm from scratch.
    pub fn reset(&mut self) -> Result<(), ResetError> {
        self.algorithm_bridge
            .pin_mut()
            .reset()
            .map_err(ResetError::Internal)
    }
}

/// Handle returned by [`Algorithm::compute`] giving access to the
/// algorithm's outputs.
///
/// Two lifetimes are at play:
///
/// * `'algorithm` — the lifetime of the [`Essentia`] handle that owns the
///   underlying global state. All outputs read through this result must
///   not outlive that handle.
/// * `'result` — the lifetime of the borrow against the algorithm's own
///   internal output buffers. Reading an output borrows for `'result`,
///   which is bounded by how long the [`ComputeResult`] is in scope.
///
/// In the generated builders, both lifetimes are propagated automatically
/// so end users rarely have to think about them.
pub struct ComputeResult<'algorithm, 'result> {
    /// Borrow against the algorithm whose buffers we are reading from.
    algorithm: &'result Algorithm<'algorithm, Configured>,
}

impl<'algorithm, 'result> ComputeResult<'algorithm, 'result> {
    /// Read a typed output by name.
    ///
    /// Validates the output against introspection in the same two ways as
    /// inputs/parameters: the output must exist on this algorithm, and the
    /// static type `T` must match the output's declared type.
    ///
    /// The returned [`DataContainer`] borrows from the algorithm — so the
    /// underlying C++ buffer is not copied. Convert it to a Rust value with
    /// [`GetFromDataContainer::get`](crate::GetFromDataContainer::get).
    pub fn output<T>(&self, key: &str) -> Result<DataContainer<'result, T>, OutputError>
    where
        T: InputOutputData + HasDataType,
    {
        let output_info = self
            .algorithm
            .introspection
            .get_output(key)
            .ok_or_else(|| OutputError::OutputNotFound {
                output: key.to_string(),
            })?;

        let expected_type = T::data_type();
        let output_data_type = output_info.input_output_type();

        if output_data_type != expected_type {
            return Err(OutputError::TypeMismatch {
                output: key.to_string(),
                expected: expected_type,
                actual: output_data_type,
            });
        }

        // The introspection check above guarantees this call cannot fail
        // for "output not found" reasons; any error here would be a bug.
        let data_container = self
            .algorithm
            .algorithm_bridge
            .get_output(key)
            .map(|ffi_data_container| DataContainer::new_borrowed(ffi_data_container))
            .expect(&format!("failed to get output '{}' after validation", key));

        Ok(data_container)
    }
}
