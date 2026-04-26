//! [`DataContainer<'a, T>`] — typed handle to an Essentia value.
//!
//! Every value flowing into or out of an Essentia algorithm — parameters,
//! inputs, outputs, pool entries — is internally a C++ tagged variant. This
//! file wraps that variant in a Rust struct that carries:
//!
//! 1. Either ownership of the underlying C++ object (via [`cxx::UniquePtr`])
//!    or a borrow against an FFI value owned by an algorithm.
//! 2. A *phantom* type marker `T` (one of the structs in
//!    [`data_type`](super::types::data_type)) that records what the C++
//!    payload is supposed to be.
//!
//! The phantom marker means a `DataContainer<'_, data_type::Float>` and a
//! `DataContainer<'_, data_type::VectorFloat>` are different Rust types even
//! though their FFI representations are byte-identical, so the type checker
//! can prevent mixing them up at compile time.

use cxx::UniquePtr;
use essentia_sys::ffi;
use std::marker::PhantomData;
use thiserror::Error;

use super::types::{DataType, HasDataType};

/// Either an owned or a borrowed FFI container.
///
/// Outputs of an algorithm are owned by the algorithm itself, so reading them
/// from Rust produces a *borrow* tied to the lifetime of the algorithm. Inputs
/// and parameters created in Rust are *owned* by the [`UniquePtr`] until they
/// are handed off to C++.
pub enum DataContainerInner<'a> {
    /// We own the underlying C++ object outright.
    Owned(UniquePtr<ffi::DataContainer>),
    /// We hold a borrow into a C++ object owned by something else (typically
    /// an `AlgorithmBridge`).
    Borrowed(&'a ffi::DataContainer),
}

impl<'a> AsRef<ffi::DataContainer> for DataContainerInner<'a> {
    fn as_ref(&self) -> &ffi::DataContainer {
        match self {
            DataContainerInner::Owned(ptr) => ptr.as_ref().expect("UniquePtr should not be null"),
            DataContainerInner::Borrowed(reference) => reference,
        }
    }
}

/// Typed handle to an Essentia value.
///
/// `T` is one of the zero-sized markers from
/// [`data_type`](super::types::data_type) (e.g.
/// [`data_type::Float`](super::types::data_type::Float),
/// [`data_type::VectorFloat`](super::types::data_type::VectorFloat)) and is
/// purely a compile-time tag — there's no runtime cost to instantiating it.
///
/// The `'a` lifetime is non-trivial: outputs returned from a `compute` call
/// borrow from the algorithm that produced them, so they cannot outlive that
/// algorithm. Owned containers (created by user code) use `'static`.
pub struct DataContainer<'a, T> {
    /// Either an owned C++ object or a borrow into one. Crate-private so
    /// that the [`Owned`](DataContainerInner::Owned) /
    /// [`Borrowed`](DataContainerInner::Borrowed) split stays an
    /// implementation detail.
    pub(crate) inner: DataContainerInner<'a>,
    /// Phantom marker recording the compile-time type tag.
    _marker: PhantomData<T>,
}

impl<'a, T> DataContainer<'a, T> {
    /// Wrap an owned C++ container.
    ///
    /// Crate-private — user code creates owned containers via the
    /// [`IntoDataContainer`](crate::IntoDataContainer) trait, which knows how
    /// to call the right `ffi::create_data_container_from_…` constructor for
    /// each Rust type.
    pub(crate) fn new_owned(inner: UniquePtr<ffi::DataContainer>) -> Self {
        Self {
            inner: DataContainerInner::Owned(inner),
            _marker: PhantomData,
        }
    }

    /// Wrap a borrowed C++ container.
    ///
    /// Crate-private — typically used internally to expose an algorithm's
    /// output without copying.
    pub(crate) fn new_borrowed(inner: &'a ffi::DataContainer) -> Self {
        Self {
            inner: DataContainerInner::Borrowed(inner),
            _marker: PhantomData,
        }
    }

    /// Erase the compile-time type marker, turning this into a
    /// `DataContainer<Any>`.
    ///
    /// Useful when the static type can't be expressed (heterogeneous
    /// collections, dynamic dispatch, …). The runtime tag is still available
    /// via [`Self::data_type`].
    pub fn into_any(self) -> DataContainer<'a, super::types::data_type::Any> {
        DataContainer {
            inner: self.inner,
            _marker: PhantomData,
        }
    }

    /// Read the runtime type tag stored in the underlying C++ container.
    ///
    /// For statically typed containers this should always agree with
    /// `T::data_type()` (and there is a [`Self::verify_type`] helper for
    /// asserting that). It can disagree only after a type-erasing
    /// [`Self::into_any`] step or in code that builds containers manually.
    pub fn data_type(&self) -> DataType {
        self.inner.as_ref().get_data_type().into()
    }

    /// Take ownership of the underlying C++ object.
    ///
    /// If the container is already owned this is a zero-cost move. If it is
    /// borrowed, the data is **deep-copied** into a fresh owned container by
    /// dispatching on the runtime tag (see [`copy_to_owned`]).
    ///
    /// This is the conversion used when an owned input has to be handed to a
    /// `set_input` / `add` / `set` call on the C++ side.
    pub fn into_owned_ptr(self) -> UniquePtr<ffi::DataContainer> {
        match self.inner {
            DataContainerInner::Owned(ptr) => ptr,
            DataContainerInner::Borrowed(borrowed) => copy_to_owned(borrowed),
        }
    }
}

impl<'a, T: HasDataType> DataContainer<'a, T> {
    /// The compile-time [`DataType`] this container claims to hold.
    ///
    /// Pure type-level lookup; equivalent to `T::data_type()`.
    pub fn compile_time_data_type() -> DataType {
        T::data_type()
    }

    /// Sanity-check that the runtime tag matches the compile-time marker.
    ///
    /// Returns [`TypeMismatchError`] if the container's actual `DataType`
    /// disagrees with what `T` says it should be — this can only happen
    /// after a [`into_any`](Self::into_any) round-trip or if user code
    /// manually constructed an inconsistent container.
    pub fn verify_type(&self) -> Result<(), TypeMismatchError> {
        let runtime_type = self.data_type();
        let compile_time_type = Self::compile_time_data_type();

        if runtime_type == compile_time_type {
            Ok(())
        } else {
            Err(TypeMismatchError {
                expected: compile_time_type,
                actual: runtime_type,
            })
        }
    }
}

/// Returned by [`DataContainer::verify_type`] when the runtime tag and the
/// compile-time marker disagree.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("Type mismatch: expected {expected}, got {actual}")]
pub struct TypeMismatchError {
    /// What [`HasDataType`] said `T` should be.
    pub expected: DataType,
    /// What the C++ container actually carries.
    pub actual: DataType,
}

/// Deep-copy a borrowed FFI container into a freshly-owned one, dispatching
/// on its runtime [`DataType`].
///
/// Used by [`DataContainer::into_owned_ptr`] for the borrowed case. Every
/// supported [`DataType`] must have a branch here, otherwise the function
/// panics. Adding a new data type to the project therefore requires updating
/// this match — the panic message is intentionally explicit.
fn copy_to_owned(data: &ffi::DataContainer) -> UniquePtr<ffi::DataContainer> {
    let data_type = data.get_data_type();

    match data_type {
        ffi::DataType::Bool => {
            let value = data.get_bool().unwrap();
            ffi::create_data_container_from_bool(value)
        }
        ffi::DataType::String => {
            let value = data.get_string().unwrap();
            ffi::create_data_container_from_string(&value)
        }
        ffi::DataType::Float => {
            let value = data.get_float().unwrap();
            ffi::create_data_container_from_float(value)
        }
        ffi::DataType::Int => {
            let value = data.get_int().unwrap();
            ffi::create_data_container_from_int(value)
        }
        ffi::DataType::UnsignedInt => {
            let value = data.get_unsigned_int().unwrap();
            ffi::create_data_container_from_unsigned_int(value)
        }
        ffi::DataType::Long => {
            let value = data.get_long().unwrap();
            ffi::create_data_container_from_long(value)
        }
        ffi::DataType::StereoSample => {
            let value = data.get_stereo_sample().unwrap();
            ffi::create_data_container_from_stereo_sample(value)
        }
        ffi::DataType::VectorBool => {
            let value = data.get_vector_bool().unwrap();
            ffi::create_data_container_from_vector_bool(&value)
        }
        ffi::DataType::VectorInt => {
            let value = data.get_vector_int().unwrap();
            ffi::create_data_container_from_vector_int(value)
        }
        ffi::DataType::VectorString => {
            let strings = data.get_vector_string().unwrap();
            let str_refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
            ffi::create_data_container_from_vector_string(&str_refs)
        }
        ffi::DataType::VectorFloat => {
            let value = data.get_vector_float().unwrap();
            ffi::create_data_container_from_vector_float(value)
        }
        ffi::DataType::VectorStereoSample => {
            let value = data.get_vector_stereo_sample().unwrap();
            ffi::create_data_container_from_vector_stereo_sample(value)
        }
        ffi::DataType::VectorVectorFloat => {
            let value = data.get_vector_vector_float().unwrap();
            ffi::create_data_container_from_vector_vector_float(value)
        }
        ffi::DataType::MatrixFloat => {
            let value = data.get_matrix_float().unwrap();
            ffi::create_data_container_from_matrix_float(value)
        }
        ffi::DataType::VectorVectorString => {
            let value = data.get_vector_vector_string().unwrap();
            ffi::create_data_container_from_vector_vector_string(value)
        }
        ffi::DataType::VectorVectorStereoSample => {
            let value = data.get_vector_vector_stereo_sample().unwrap();
            ffi::create_data_container_from_vector_vector_stereo_sample(value)
        }
        ffi::DataType::VectorMatrixFloat => {
            let value = data.get_vector_matrix_float().unwrap();
            ffi::create_data_container_from_vector_matrix_float(value)
        }
        ffi::DataType::MapVectorFloat => {
            let value = data.get_map_vector_float().unwrap();
            ffi::create_data_container_from_map_vector_float(value)
        }
        ffi::DataType::MapVectorString => {
            let value = data.get_map_vector_string().unwrap();
            ffi::create_data_container_from_map_vector_string(value)
        }
        ffi::DataType::MapVectorInt => {
            let value = data.get_map_vector_int().unwrap();
            ffi::create_data_container_from_map_vector_int(value)
        }
        ffi::DataType::MapFloat => {
            let value = data.get_map_float().unwrap();
            ffi::create_data_container_from_map_float(value)
        }
        ffi::DataType::Pool => {
            // Pools are reference-counted on the C++ side; cloning the
            // bridge and re-wrapping it as a DataContainer is the correct
            // way to get an owned copy.
            let pool_bridge_ref = data.get_pool();
            let cloned_pool = pool_bridge_ref.clone();
            ffi::create_data_container_from_pool(cloned_pool)
        }
        data_type => {
            panic!(
                "Unsupported data type: {:?}. This indicates a bug - the Rust code is out of sync with the C++ data types.",
                data_type
            )
        }
    }
}
