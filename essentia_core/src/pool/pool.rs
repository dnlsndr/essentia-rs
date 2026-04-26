//! [`Pool`] — Essentia's heterogeneous key/value store.
//!
//! Pools are the standard way Essentia algorithms produce or consume large
//! collections of named values. They are heavily used in feature-extraction
//! pipelines where many algorithms write summary statistics ("mean MFCC",
//! "spectral centroid mean", "tempo confidence", …) into a shared pool
//! that is later serialised or fed into a downstream classifier.
//!
//! On the C++ side a Pool is essentially a `Map<String, std::variant<…>>`
//! that accepts a fixed subset of the [`DataType`]s supported by the rest
//! of the API. The same restrictions are mirrored in Rust by the
//! [`PoolData`] capability trait — [`Pool::set`] and [`Pool::get`] only
//! accept marker types that implement it (see
//! [`crate::data::constraints`]).

use cxx::UniquePtr;
use essentia_sys::ffi;
use thiserror::Error;

use crate::IntoDataContainer;
use crate::data::types::HasDataType;
use crate::data::{DataContainer, DataType, GetFromDataContainer, PoolData};

/// A heterogeneous key/value store.
///
/// Keys are arbitrary [`String`]s. Values are typed via the [`PoolData`]
/// capability trait — only a small subset of Essentia's [`DataType`]s are
/// permitted as pool values (currently `Float`, `String`, `StereoSample`,
/// `VectorFloat`, `VectorString`, `VectorStereoSample`, `TensorFloat`).
///
/// `Pool` itself can be passed to algorithms as an input or output — see
/// the [`IntoDataContainer<data_type::Pool>`] impl in
/// [`conversion_into`](crate::data) and the [`GetFromDataContainer<Pool>`]
/// impl in [`conversion_get`](crate::data).
pub struct Pool {
    /// Owned C++ pool bridge.
    inner: UniquePtr<ffi::PoolBridge>,
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

impl Pool {
    /// Construct a new, empty pool.
    pub fn new() -> Self {
        Self {
            inner: ffi::create_pool_bridge(),
        }
    }

    /// Wrap an existing FFI bridge as a Rust [`Pool`].
    ///
    /// Crate-private, used by the conversion machinery to materialise pools
    /// returned by `compute()`.
    pub(crate) fn new_from_bridge(bridge: UniquePtr<ffi::PoolBridge>) -> Self {
        Self { inner: bridge }
    }

    /// Insert (or overwrite) `value` under `key`.
    ///
    /// The static `T` is one of the markers permitted by [`PoolData`] — a
    /// type the C++ side knows how to store in a pool. The current
    /// implementation never returns `Err`, but the signature reserves the
    /// option for a future surface (e.g. validating duplicate-key
    /// behaviour).
    pub fn set<T>(&mut self, key: &str, value: impl IntoDataContainer<T>) -> Result<(), PoolError>
    where
        T: PoolData + HasDataType,
    {
        let data_container = value.into_data_container();

        self.inner
            .pin_mut()
            .set(key, data_container.into_owned_ptr());

        Ok(())
    }

    /// Read a value out of the pool, converting it to the Rust type `R`.
    ///
    /// The static `T` is the pool data marker (e.g.
    /// [`data_type::VectorFloat`](crate::data_type::VectorFloat)); the
    /// blanket `R` is the destination Rust type (e.g. `Vec<f32>`). The
    /// `where` clause says: there must be a way to read a `T`-typed
    /// container as an `R`.
    ///
    /// Errors:
    ///
    /// * [`PoolError::KeyNotFound`] — the key isn't present.
    /// * [`PoolError::TypeMismatch`] — the key exists but holds a
    ///   different type than `T`.
    /// * [`PoolError::Internal`] — the C++ side raised an exception while
    ///   reading.
    pub fn get<T, R>(&self, key: &str) -> Result<R, PoolError>
    where
        T: PoolData + HasDataType,
        for<'a> DataContainer<'a, T>: GetFromDataContainer<R>,
    {
        if !self.contains(key) {
            return Err(PoolError::KeyNotFound {
                key: key.to_string(),
            });
        }

        let data_container_ffi =
            self.inner
                .as_ref()
                .unwrap()
                .get(key)
                .map_err(|exception| PoolError::Internal {
                    key: key.to_string(),
                    source: exception,
                })?;

        let data_container = DataContainer::new_borrowed(data_container_ffi.as_ref().unwrap());

        // Verify type safety at runtime (backup to compile-time checks)
        let expected_type = T::data_type();
        let actual_type = data_container.data_type();

        if actual_type != expected_type {
            return Err(PoolError::TypeMismatch {
                key: key.to_string(),
                expected: expected_type,
                actual: actual_type,
            });
        }

        Ok(data_container.get())
    }

    /// Read a value out of the pool as a [`DataContainer`].
    ///
    /// Like [`Self::get`], but returns the raw typed container instead of
    /// converting to a Rust type. Useful when you want to forward the
    /// value to another algorithm without paying the conversion cost.
    pub fn get_container<T>(&self, key: &str) -> Result<DataContainer<'static, T>, PoolError>
    where
        T: PoolData + HasDataType,
    {
        if !self.contains(key) {
            return Err(PoolError::KeyNotFound {
                key: key.to_string(),
            });
        }

        let data_container_ffi =
            self.inner
                .as_ref()
                .unwrap()
                .get(key)
                .map_err(|exception| PoolError::Internal {
                    key: key.to_string(),
                    source: exception,
                })?;

        let data_container = DataContainer::new_owned(data_container_ffi);

        // Verify type safety
        let expected_type = T::data_type();
        let actual_type = data_container.data_type();

        if actual_type != expected_type {
            return Err(PoolError::TypeMismatch {
                key: key.to_string(),
                expected: expected_type,
                actual: actual_type,
            });
        }

        Ok(data_container)
    }

    /// `true` if `key` exists in the pool.
    pub fn contains(&self, key: &str) -> bool {
        self.inner.as_ref().unwrap().contains(key)
    }

    /// Snapshot of every key currently stored in the pool, in unspecified
    /// order.
    pub fn keys(&self) -> Vec<String> {
        self.inner.as_ref().unwrap().keys()
    }

    /// Number of keys in the pool.
    pub fn len(&self) -> usize {
        self.keys().len()
    }

    /// `true` if the pool has no keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Consume the pool and surrender its underlying FFI bridge.
    ///
    /// Crate-private, used when handing a pool to an algorithm as an
    /// input/output value (where the FFI side takes ownership).
    pub(crate) fn into_owned_ptr(self) -> UniquePtr<ffi::PoolBridge> {
        self.inner
    }
}

/// Errors returned by [`Pool`] read methods.
///
/// Writes (`set`) currently never fail and so do not have a dedicated
/// variant.
#[derive(Debug, Error)]
pub enum PoolError {
    /// The pool has no entry under that key.
    #[error("Key '{key}' not found in pool")]
    KeyNotFound { key: String },

    /// The key exists but the stored value has a different type than the
    /// caller asked for.
    #[error("Type mismatch for key '{key}': expected {expected}, found {actual}")]
    TypeMismatch {
        key: String,
        /// Type the caller requested.
        expected: DataType,
        /// Type the value actually has.
        actual: DataType,
    },

    /// The C++ side raised an exception while reading the value.
    #[error("Internal error for key '{key}': {source}")]
    Internal {
        key: String,
        #[source]
        source: cxx::Exception,
    },
}
