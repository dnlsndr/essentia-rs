//! Rust → [`DataContainer`] conversions.
//!
//! Idiomatic Rust types (`f32`, `&[f32]`, `&Array2<f32>`, `&HashMap<String,
//! Vec<f32>>`, …) do not match the FFI's variant payload layout one-to-one
//! — vectors of vectors, slice-of-slice maps, tensors with shape arrays
//! and the like all need adapters on the way down. This file holds those
//! adapters as implementations of two traits:
//!
//! * [`IntoDataContainer<T>`] — infallible conversions. The associated
//!   marker `T` (one of the structs in
//!   [`data_type`](crate::data::data_type)) constrains *which* container
//!   type the source value will produce, so a single source type can have
//!   multiple impls (e.g. a `&[Vec<f32>]` can become a
//!   [`VectorVectorFloat`](crate::data::data_type::VectorVectorFloat)
//!   directly, or a [`MatrixFloat`](crate::data::data_type::MatrixFloat)
//!   via the fallible [`TryIntoDataContainer`]).
//! * [`TryIntoDataContainer<T>`] — fallible variants for conversions that
//!   may need to validate shape (e.g. enforcing a rectangular matrix).

use essentia_sys::ffi;
use ndarray::{Array2, Array4};
use std::collections::HashMap;

use crate::{ConversionError, DataContainer, data_type};

/// Convert `self` into a typed [`DataContainer<'static, T>`] without
/// allocation failure.
///
/// `T` is a marker from [`data_type`] that determines the output container's
/// type. Multiple impls per source type are allowed — picking the right
/// container shape is the caller's job (in practice the generated builder
/// methods do that automatically).
pub trait IntoDataContainer<T> {
    /// Perform the conversion. The result owns its underlying C++ object,
    /// hence `'static`.
    fn into_data_container(self) -> DataContainer<'static, T>;
}

/// Like [`IntoDataContainer`], but the conversion may fail (e.g. when
/// validating the shape of a non-rectangular matrix).
pub trait TryIntoDataContainer<T> {
    /// Perform the conversion or return a [`ConversionError`] explaining
    /// why the input is unsuitable.
    fn try_into_data_container(self) -> Result<DataContainer<'static, T>, ConversionError>;
}

// A `DataContainer<T>` is trivially convertible to itself; the impl handles
// the borrowed-vs-owned case by deep-copying when necessary.
impl<'a, T> IntoDataContainer<T> for DataContainer<'a, T> {
    fn into_data_container(self) -> DataContainer<'static, T> {
        let owned_ptr = self.into_owned_ptr();
        DataContainer::new_owned(owned_ptr)
    }
}

// -- Scalars --------------------------------------------------------------

impl IntoDataContainer<data_type::Bool> for bool {
    fn into_data_container(self) -> DataContainer<'static, data_type::Bool> {
        DataContainer::new_owned(ffi::create_data_container_from_bool(self))
    }
}

impl IntoDataContainer<data_type::String> for &str {
    fn into_data_container(self) -> DataContainer<'static, data_type::String> {
        DataContainer::new_owned(ffi::create_data_container_from_string(self))
    }
}

impl IntoDataContainer<data_type::Int> for i32 {
    fn into_data_container(self) -> DataContainer<'static, data_type::Int> {
        DataContainer::new_owned(ffi::create_data_container_from_int(self))
    }
}

impl IntoDataContainer<data_type::Float> for f32 {
    fn into_data_container(self) -> DataContainer<'static, data_type::Float> {
        DataContainer::new_owned(ffi::create_data_container_from_float(self))
    }
}

impl IntoDataContainer<data_type::UnsignedInt> for u32 {
    fn into_data_container(self) -> DataContainer<'static, data_type::UnsignedInt> {
        DataContainer::new_owned(ffi::create_data_container_from_unsigned_int(self))
    }
}

impl IntoDataContainer<data_type::Long> for i64 {
    fn into_data_container(self) -> DataContainer<'static, data_type::Long> {
        DataContainer::new_owned(ffi::create_data_container_from_long(self))
    }
}

impl IntoDataContainer<data_type::StereoSample> for ffi::StereoSample {
    fn into_data_container(self) -> DataContainer<'static, data_type::StereoSample> {
        DataContainer::new_owned(ffi::create_data_container_from_stereo_sample(self))
    }
}

impl IntoDataContainer<data_type::Complex> for num::Complex<f32> {
    fn into_data_container(self) -> DataContainer<'static, data_type::Complex> {
        // num::Complex and ffi::Complex have the same fields but different
        // types; the FFI requires the C++-side struct so we re-pack here.
        DataContainer::new_owned(ffi::create_data_container_from_complex(ffi::Complex {
            real: self.re,
            imag: self.im,
        }))
    }
}

/// Tensor conversion. The C++ side expects a flat slice plus an explicit
/// shape array; ndarray exposes both via `as_slice` and `shape`.
impl IntoDataContainer<data_type::TensorFloat> for &Array4<f32> {
    fn into_data_container(self) -> DataContainer<'static, data_type::TensorFloat> {
        let slice = self.as_slice().expect("Array must be contiguous");
        let shape = [
            self.shape()[0],
            self.shape()[1],
            self.shape()[2],
            self.shape()[3],
        ];

        DataContainer::new_owned(ffi::create_data_container_from_tensor_float(
            ffi::TensorFloat {
                slice,
                shape: &shape,
            },
        ))
    }
}

// -- Flat vectors ---------------------------------------------------------

impl IntoDataContainer<data_type::VectorBool> for &[bool] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorBool> {
        DataContainer::new_owned(ffi::create_data_container_from_vector_bool(self))
    }
}

impl IntoDataContainer<data_type::VectorInt> for &[i32] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorInt> {
        DataContainer::new_owned(ffi::create_data_container_from_vector_int(self))
    }
}

impl IntoDataContainer<data_type::VectorString> for &[&str] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorString> {
        DataContainer::new_owned(ffi::create_data_container_from_vector_string(self))
    }
}

impl IntoDataContainer<data_type::VectorFloat> for &[f32] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorFloat> {
        DataContainer::new_owned(ffi::create_data_container_from_vector_float(self))
    }
}

impl IntoDataContainer<data_type::VectorStereoSample> for &[ffi::StereoSample] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorStereoSample> {
        DataContainer::new_owned(ffi::create_data_container_from_vector_stereo_sample(self))
    }
}

impl IntoDataContainer<data_type::VectorComplex> for &[num::Complex<f32>] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorComplex> {
        // Element-wise re-pack from `num::Complex` to `ffi::Complex`.
        let ffi_vec: Vec<ffi::Complex> = self
            .iter()
            .map(|c| ffi::Complex {
                real: c.re,
                imag: c.im,
            })
            .collect();
        DataContainer::new_owned(ffi::create_data_container_from_vector_complex(&ffi_vec))
    }
}

// -- Nested vectors -------------------------------------------------------

impl IntoDataContainer<data_type::VectorVectorFloat> for &[Vec<f32>] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorVectorFloat> {
        // The cxx bridge cannot carry a `&[&[f32]]` directly because each
        // inner slice needs its own SliceFloat wrapper. We project each Vec
        // into a SliceFloat without copying the underlying data.
        DataContainer::new_owned(ffi::create_data_container_from_vector_vector_float(
            self.iter()
                .map(|item| ffi::SliceFloat {
                    slice: item.as_slice(),
                })
                .collect(),
        ))
    }
}

/// 2-D ndarray → matrix container. Requires contiguous memory layout
/// (which is the default for `Array2`).
impl IntoDataContainer<data_type::MatrixFloat> for &Array2<f32> {
    fn into_data_container(self) -> DataContainer<'static, data_type::MatrixFloat> {
        let slice = self.as_slice().expect("Array must be contiguous");
        let (dim1, dim2) = self.dim();

        DataContainer::new_owned(ffi::create_data_container_from_matrix_float(
            ffi::MatrixFloat { slice, dim1, dim2 },
        ))
    }
}

impl IntoDataContainer<data_type::VectorVectorString> for &[&[&str]] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorVectorString> {
        // Each inner row must own its strings to fit the cxx bridge shape;
        // we copy the &str values into owned `String`s here.
        DataContainer::new_owned(ffi::create_data_container_from_vector_vector_string(
            self.iter()
                .map(|item| ffi::VecString {
                    vec: item.iter().map(|s| s.to_string()).collect(),
                })
                .collect(),
        ))
    }
}

impl IntoDataContainer<data_type::VectorVectorStereoSample> for &[&[ffi::StereoSample]] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorVectorStereoSample> {
        DataContainer::new_owned(ffi::create_data_container_from_vector_vector_stereo_sample(
            self.iter()
                .map(|item| ffi::SliceStereoSample { slice: item })
                .collect(),
        ))
    }
}

impl IntoDataContainer<data_type::VectorVectorComplex> for &[Vec<num::Complex<f32>>] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorVectorComplex> {
        // Element-wise repack of each inner Vec into ffi::Complex form.
        DataContainer::new_owned(ffi::create_data_container_from_vector_vector_complex(
            self.iter()
                .map(|item| ffi::VecComplex {
                    vec: item
                        .iter()
                        .map(|c| ffi::Complex {
                            real: c.re,
                            imag: c.im,
                        })
                        .collect(),
                })
                .collect(),
        ))
    }
}

impl IntoDataContainer<data_type::VectorMatrixFloat> for &[Array2<f32>] {
    fn into_data_container(self) -> DataContainer<'static, data_type::VectorMatrixFloat> {
        DataContainer::new_owned(ffi::create_data_container_from_vector_matrix_float(
            self.iter()
                .map(|array| {
                    let slice = array.as_slice().expect("Array must be contiguous");
                    let (dim1, dim2) = array.dim();
                    ffi::MatrixFloat { slice, dim1, dim2 }
                })
                .collect(),
        ))
    }
}

// -- Maps ------------------------------------------------------------------
// HashMap → variant-of-map-entries. The cxx bridge expects a flat
// `Vec<MapEntry…>` whose elements own a `String` key and (depending on the
// variant) borrow or own their value. We materialise that vec here.

impl IntoDataContainer<data_type::MapVectorFloat> for &HashMap<String, Vec<f32>> {
    fn into_data_container(self) -> DataContainer<'static, data_type::MapVectorFloat> {
        DataContainer::new_owned(ffi::create_data_container_from_map_vector_float(
            self.iter()
                .map(|(key, vec)| ffi::MapEntryVectorFloat {
                    key: key.clone(),
                    value: vec.as_slice(),
                })
                .collect(),
        ))
    }
}

impl IntoDataContainer<data_type::MapVectorString> for &HashMap<String, Vec<String>> {
    fn into_data_container(self) -> DataContainer<'static, data_type::MapVectorString> {
        DataContainer::new_owned(ffi::create_data_container_from_map_vector_string(
            self.iter()
                .map(|(key, vec)| ffi::MapEntryVectorString {
                    key: key.clone(),
                    value: vec.clone(),
                })
                .collect(),
        ))
    }
}

impl IntoDataContainer<data_type::MapVectorInt> for &HashMap<String, Vec<i32>> {
    fn into_data_container(self) -> DataContainer<'static, data_type::MapVectorInt> {
        DataContainer::new_owned(ffi::create_data_container_from_map_vector_int(
            self.iter()
                .map(|(key, vec)| ffi::MapEntryVectorInt {
                    key: key.clone(),
                    value: vec.as_slice(),
                })
                .collect(),
        ))
    }
}

impl IntoDataContainer<data_type::MapVectorComplex> for &HashMap<String, Vec<num::Complex<f32>>> {
    fn into_data_container(self) -> DataContainer<'static, data_type::MapVectorComplex> {
        // Two passes: first materialise owned Vec<ffi::Complex> per key
        // (so the slices in the second pass can borrow from them), then
        // assemble the borrowing entry list the cxx bridge actually wants.
        let converted_data: Vec<(String, Vec<ffi::Complex>)> = self
            .iter()
            .map(|(key, vec)| {
                (
                    key.clone(),
                    vec.iter()
                        .map(|c| ffi::Complex {
                            real: c.re,
                            imag: c.im,
                        })
                        .collect(),
                )
            })
            .collect();

        let entries: Vec<ffi::MapEntryVectorComplex> = converted_data
            .iter()
            .map(|(key, ffi_vec)| ffi::MapEntryVectorComplex {
                key: key.clone(),
                value: ffi_vec.as_slice(),
            })
            .collect();

        DataContainer::new_owned(ffi::create_data_container_from_map_vector_complex(entries))
    }
}

impl IntoDataContainer<data_type::MapFloat> for &HashMap<String, f32> {
    fn into_data_container(self) -> DataContainer<'static, data_type::MapFloat> {
        DataContainer::new_owned(ffi::create_data_container_from_map_float(
            self.iter()
                .map(|(key, &val)| ffi::MapEntryFloat {
                    key: key.clone(),
                    value: val,
                })
                .collect(),
        ))
    }
}

/// Try to interpret a slice of `Vec<f32>` rows as a 2-D matrix.
///
/// Fails if the slice is empty, if any row is empty, or if rows have
/// different lengths. This is the only `MatrixFloat` constructor exposed
/// to user code that does not require an [`Array2`] up-front.
impl TryIntoDataContainer<data_type::MatrixFloat> for &[Vec<f32>] {
    fn try_into_data_container(
        self,
    ) -> Result<DataContainer<'static, data_type::MatrixFloat>, ConversionError> {
        if self.is_empty() {
            return Err(ConversionError::InvalidFormat {
                message: "Cannot create matrix from empty vector".to_string(),
            });
        }

        let expected_cols = self[0].len();
        if expected_cols == 0 {
            return Err(ConversionError::InvalidFormat {
                message: "Cannot create matrix from empty rows".to_string(),
            });
        }

        for (row_idx, row) in self.iter().enumerate() {
            if row.len() != expected_cols {
                return Err(ConversionError::InvalidFormat {
                    message: format!(
                        "Non-rectangular matrix: row {} has {} elements, expected {}",
                        row_idx,
                        row.len(),
                        expected_cols
                    ),
                });
            }
        }

        let mut flat_data = Vec::with_capacity(self.len() * expected_cols);
        for row in self {
            flat_data.extend(row);
        }

        let dim1 = flat_data.len() / expected_cols;
        let dim2 = expected_cols;

        Ok(DataContainer::new_owned(
            ffi::create_data_container_from_matrix_float(ffi::MatrixFloat {
                slice: &flat_data,
                dim1,
                dim2,
            }),
        ))
    }
}

/// Hand a [`Pool`](crate::pool::Pool) to an algorithm as an input/output.
///
/// Consumes the pool because the underlying C++ object is moved into the
/// container.
impl IntoDataContainer<data_type::Pool> for crate::pool::Pool {
    fn into_data_container(self) -> DataContainer<'static, data_type::Pool> {
        DataContainer::new_owned(ffi::create_data_container_from_pool(self.into_owned_ptr()))
    }
}
