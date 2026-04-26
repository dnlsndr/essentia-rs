//! [`DataContainer`] → Rust conversions.
//!
//! Mirror of `conversion_into.rs` for the read direction. Algorithm outputs
//! and pool reads come back as a [`DataContainer<T>`]; these traits unwrap
//! that into idiomatic Rust types.
//!
//! Two traits cover the spectrum:
//!
//! * [`GetFromDataContainer<T>`] — infallible read, used when the static `T`
//!   marker on the container guarantees a successful conversion.
//! * [`TryGetFromDataContainer<T>`] — fallible read, used when the
//!   conversion can fail at runtime (e.g. interpreting a
//!   [`VectorVectorFloat`](crate::data::data_type::VectorVectorFloat) as
//!   a 2-D `Array2` requires the rows to be rectangular).

use essentia_sys::ffi;
use ndarray::{Array2, Array4};
use std::collections::HashMap;

use crate::{ConversionError, DataContainer, Pool, data_type};

/// Read the value out of a typed container into the Rust type `T`.
///
/// The blanket parameter `T` is the **target** Rust type; the impl is
/// chosen by the static marker on the source `DataContainer<…>`. So a
/// `DataContainer<data_type::Float>` has only one impl
/// (`GetFromDataContainer<f32>`), whereas a
/// `DataContainer<data_type::VectorVectorFloat>` has multiple impls
/// (`Vec<Vec<f32>>`, `Array2<f32>` via `TryGetFromDataContainer`, …).
pub trait GetFromDataContainer<T> {
    /// Perform the conversion. Should never fail in practice — any failure
    /// would indicate the C++ side returned a value of the wrong type, in
    /// which case there is a bug elsewhere.
    fn get(&self) -> T;
}

/// Like [`GetFromDataContainer`], but the conversion can fail (typically
/// because the source data does not satisfy the target's shape constraints).
pub trait TryGetFromDataContainer<T> {
    /// Perform the conversion or return a [`ConversionError`].
    fn try_get(&self) -> Result<T, ConversionError>;
}

// -- Scalars --------------------------------------------------------------

impl<'a> GetFromDataContainer<bool> for DataContainer<'a, data_type::Bool> {
    fn get(&self) -> bool {
        self.inner.as_ref().get_bool().unwrap()
    }
}

impl<'a> GetFromDataContainer<String> for DataContainer<'a, data_type::String> {
    fn get(&self) -> String {
        self.inner.as_ref().get_string().unwrap().to_string()
    }
}

impl<'a> GetFromDataContainer<i32> for DataContainer<'a, data_type::Int> {
    fn get(&self) -> i32 {
        self.inner.as_ref().get_int().unwrap()
    }
}

impl<'a> GetFromDataContainer<f32> for DataContainer<'a, data_type::Float> {
    fn get(&self) -> f32 {
        self.inner.as_ref().get_float().unwrap()
    }
}

impl<'a> GetFromDataContainer<u32> for DataContainer<'a, data_type::UnsignedInt> {
    fn get(&self) -> u32 {
        self.inner.as_ref().get_unsigned_int().unwrap()
    }
}

impl<'a> GetFromDataContainer<i64> for DataContainer<'a, data_type::Long> {
    fn get(&self) -> i64 {
        self.inner.as_ref().get_long().unwrap()
    }
}

impl<'a> GetFromDataContainer<ffi::StereoSample> for DataContainer<'a, data_type::StereoSample> {
    fn get(&self) -> ffi::StereoSample {
        self.inner.as_ref().get_stereo_sample().unwrap()
    }
}

impl<'a> GetFromDataContainer<num::Complex<f32>> for DataContainer<'a, data_type::Complex> {
    fn get(&self) -> num::Complex<f32> {
        // Repack from the FFI struct (real/imag fields) to num::Complex.
        let ffi_complex = self.inner.as_ref().get_complex().unwrap();
        num::Complex::new(ffi_complex.real, ffi_complex.imag)
    }
}

/// 4-D tensor → ndarray. The C++ side guarantees the slice's length
/// matches the product of the shape, so the unwrap is safe.
impl<'a> GetFromDataContainer<Array4<f32>> for DataContainer<'a, data_type::TensorFloat> {
    fn get(&self) -> Array4<f32> {
        let tensor = self.inner.as_ref().get_tensor_float().unwrap();

        let shape = (
            tensor.shape[0],
            tensor.shape[1],
            tensor.shape[2],
            tensor.shape[3],
        );

        Array4::from_shape_vec(shape, tensor.slice.to_vec()).unwrap() // Safe because C++ guarantees correct dimensions
    }
}

// -- Flat vectors ---------------------------------------------------------

impl<'a> GetFromDataContainer<Vec<bool>> for DataContainer<'a, data_type::VectorBool> {
    fn get(&self) -> Vec<bool> {
        self.inner.as_ref().get_vector_bool().unwrap()
    }
}

impl<'a> GetFromDataContainer<Vec<i32>> for DataContainer<'a, data_type::VectorInt> {
    fn get(&self) -> Vec<i32> {
        self.inner.as_ref().get_vector_int().unwrap().to_vec()
    }
}

impl<'a> GetFromDataContainer<Vec<String>> for DataContainer<'a, data_type::VectorString> {
    fn get(&self) -> Vec<String> {
        self.inner.as_ref().get_vector_string().unwrap()
    }
}

impl<'a> GetFromDataContainer<Vec<f32>> for DataContainer<'a, data_type::VectorFloat> {
    fn get(&self) -> Vec<f32> {
        self.inner.as_ref().get_vector_float().unwrap().to_vec()
    }
}

impl<'a> GetFromDataContainer<Vec<ffi::StereoSample>>
    for DataContainer<'a, data_type::VectorStereoSample>
{
    fn get(&self) -> Vec<ffi::StereoSample> {
        self.inner
            .as_ref()
            .get_vector_stereo_sample()
            .unwrap()
            .to_vec()
    }
}

impl<'a> GetFromDataContainer<Vec<num::Complex<f32>>>
    for DataContainer<'a, data_type::VectorComplex>
{
    fn get(&self) -> Vec<num::Complex<f32>> {
        self.inner
            .as_ref()
            .get_vector_complex()
            .unwrap()
            .iter()
            .map(|c| num::Complex::new(c.real, c.imag))
            .collect()
    }
}

// -- Matrices and vectors of matrices -------------------------------------

impl<'a> GetFromDataContainer<Array2<f32>> for DataContainer<'a, data_type::MatrixFloat> {
    fn get(&self) -> Array2<f32> {
        let matrix_float = self.inner.as_ref().get_matrix_float().unwrap();

        Array2::from_shape_vec(
            (matrix_float.dim1, matrix_float.dim2),
            matrix_float.slice.to_vec(),
        )
        .unwrap() // Safe because C++ guarantees correct dimensions
    }
}

impl<'a> GetFromDataContainer<Vec<Array2<f32>>>
    for DataContainer<'a, data_type::VectorMatrixFloat>
{
    fn get(&self) -> Vec<Array2<f32>> {
        let matrices = self.inner.as_ref().get_vector_matrix_float().unwrap();

        matrices
            .into_iter()
            .map(|matrix_float| {
                Array2::from_shape_vec(
                    (matrix_float.dim1, matrix_float.dim2),
                    matrix_float.slice.to_vec(),
                )
                .unwrap() // Safe because C++ guarantees correct dimensions
            })
            .collect()
    }
}

// -- Nested vectors -------------------------------------------------------

impl<'a> GetFromDataContainer<Vec<Vec<f32>>> for DataContainer<'a, data_type::VectorVectorFloat> {
    fn get(&self) -> Vec<Vec<f32>> {
        self.inner
            .as_ref()
            .get_vector_vector_float()
            .unwrap()
            .into_iter()
            .map(|float_slice| float_slice.slice.to_vec())
            .collect()
    }
}

/// Reinterpret a `VectorVectorFloat` as a 2-D `Array2`.
///
/// Fails if the source has zero rows, zero columns, or non-rectangular rows
/// — the same conditions as the corresponding `TryIntoDataContainer` for
/// `MatrixFloat`.
impl<'a> TryGetFromDataContainer<Array2<f32>> for DataContainer<'a, data_type::VectorVectorFloat> {
    fn try_get(&self) -> Result<Array2<f32>, ConversionError> {
        let vec_vec_data = self.inner.as_ref().get_vector_vector_float().unwrap();

        if vec_vec_data.is_empty() {
            return Err(ConversionError::InvalidFormat {
                message: "Cannot create matrix from empty vector".to_string(),
            });
        }

        let expected_cols = vec_vec_data[0].slice.len();
        if expected_cols == 0 {
            return Err(ConversionError::InvalidFormat {
                message: "Cannot create matrix from empty rows".to_string(),
            });
        }

        for (row_idx, row_data) in vec_vec_data.iter().enumerate() {
            if row_data.slice.len() != expected_cols {
                return Err(ConversionError::InvalidFormat {
                    message: format!(
                        "Non-rectangular matrix: row {} has {} elements, expected {}",
                        row_idx,
                        row_data.slice.len(),
                        expected_cols
                    ),
                });
            }
        }

        let mut flat_data = Vec::with_capacity(vec_vec_data.len() * expected_cols);
        for row_data in &vec_vec_data {
            flat_data.extend_from_slice(row_data.slice);
        }

        let dim1 = vec_vec_data.len();
        let dim2 = expected_cols;

        Ok(Array2::from_shape_vec((dim1, dim2), flat_data).unwrap())
    }
}

impl<'a> GetFromDataContainer<Vec<Vec<String>>>
    for DataContainer<'a, data_type::VectorVectorString>
{
    fn get(&self) -> Vec<Vec<String>> {
        self.inner
            .as_ref()
            .get_vector_vector_string()
            .unwrap()
            .into_iter()
            .map(|vec_string| vec_string.vec)
            .collect()
    }
}

impl<'a> GetFromDataContainer<Vec<Vec<ffi::StereoSample>>>
    for DataContainer<'a, data_type::VectorVectorStereoSample>
{
    fn get(&self) -> Vec<Vec<ffi::StereoSample>> {
        self.inner
            .as_ref()
            .get_vector_vector_stereo_sample()
            .unwrap()
            .into_iter()
            .map(|slice_stereo_sample| slice_stereo_sample.slice.to_vec())
            .collect()
    }
}

impl<'a> GetFromDataContainer<Vec<Vec<num::Complex<f32>>>>
    for DataContainer<'a, data_type::VectorVectorComplex>
{
    fn get(&self) -> Vec<Vec<num::Complex<f32>>> {
        self.inner
            .as_ref()
            .get_vector_vector_complex()
            .unwrap()
            .into_iter()
            .map(|vec_complex| {
                vec_complex
                    .vec
                    .into_iter()
                    .map(|c| num::Complex::new(c.real, c.imag))
                    .collect()
            })
            .collect()
    }
}

// -- Maps ------------------------------------------------------------------

impl<'a> GetFromDataContainer<HashMap<String, f32>> for DataContainer<'a, data_type::MapFloat> {
    fn get(&self) -> HashMap<String, f32> {
        self.inner
            .as_ref()
            .get_map_float()
            .unwrap()
            .into_iter()
            .map(|entry| (entry.key.to_string(), entry.value))
            .collect()
    }
}

impl<'a> GetFromDataContainer<HashMap<String, Vec<f32>>>
    for DataContainer<'a, data_type::MapVectorFloat>
{
    fn get(&self) -> HashMap<String, Vec<f32>> {
        self.inner
            .as_ref()
            .get_map_vector_float()
            .unwrap()
            .into_iter()
            .map(|entry| (entry.key.to_string(), entry.value.to_vec()))
            .collect()
    }
}

impl<'a> GetFromDataContainer<HashMap<String, Vec<String>>>
    for DataContainer<'a, data_type::MapVectorString>
{
    fn get(&self) -> HashMap<String, Vec<String>> {
        self.inner
            .as_ref()
            .get_map_vector_string()
            .unwrap()
            .into_iter()
            .map(|entry| (entry.key.to_string(), entry.value))
            .collect()
    }
}

impl<'a> GetFromDataContainer<HashMap<String, Vec<i32>>>
    for DataContainer<'a, data_type::MapVectorInt>
{
    fn get(&self) -> HashMap<String, Vec<i32>> {
        self.inner
            .as_ref()
            .get_map_vector_int()
            .unwrap()
            .into_iter()
            .map(|entry| (entry.key.to_string(), entry.value.to_vec()))
            .collect()
    }
}

impl<'a> GetFromDataContainer<HashMap<String, Vec<num::Complex<f32>>>>
    for DataContainer<'a, data_type::MapVectorComplex>
{
    fn get(&self) -> HashMap<String, Vec<num::Complex<f32>>> {
        self.inner
            .as_ref()
            .get_map_vector_complex()
            .unwrap()
            .into_iter()
            .map(|entry| {
                (
                    entry.key.to_string(),
                    entry
                        .value
                        .iter()
                        .map(|c| num::Complex::new(c.real, c.imag))
                        .collect(),
                )
            })
            .collect()
    }
}

/// Read a [`Pool`] back out of a container.
///
/// The resulting Pool clones the underlying C++ bridge, so it can outlive
/// the algorithm that produced it.
// TODO Maybe the Pool should be take a reference to the PoolBridge?
impl<'a> GetFromDataContainer<Pool> for DataContainer<'a, data_type::Pool> {
    fn get(&self) -> Pool {
        let pool_bridge_ref = self.inner.as_ref().get_pool();
        let cloned_bridge = pool_bridge_ref.clone();
        Pool::new_from_bridge(cloned_bridge)
    }
}
