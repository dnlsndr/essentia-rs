//! Helpers shared by the per-algorithm code-generation submodules.
//!
//! Three concerns:
//!
//! * [`sanitize_identifier_string`] — escape Rust keywords by appending an
//!   underscore so they can be used as identifiers without breaking the
//!   parser.
//! * [`string_to_doc_comment`] — turn an Essentia free-form description
//!   into a sequence of `#[doc = "…"]` attributes. We word-wrap to 80
//!   columns to keep the generated source readable.
//! * [`data_type_enum_to_data_type_marker`] — translate a
//!   [`DataType`](essentia_core::DataType) enum value into the matching
//!   compile-time marker path under `crate::data_type`.

use essentia_core::DataType;
use proc_macro2::TokenStream;
use quote::quote;
use textwrap::fill;

/// Append `_` to the input if it is a Rust keyword.
///
/// Essentia parameters use names like `type` or `mode` which clash with
/// reserved words. Without escaping, the generated code would refuse to
/// compile.
pub fn sanitize_identifier_string(string: &str) -> String {
    match string {
        "type" | "match" | "if" | "else" | "while" | "for" | "loop" | "fn" | "let" | "mut"
        | "const" | "static" | "struct" | "enum" | "impl" | "trait" | "mod" | "pub" | "use"
        | "crate" | "super" | "self" | "Self" | "as" | "where" | "move" | "ref" | "in"
        | "break" | "continue" | "return" | "yield" | "unsafe" | "extern" | "dyn" | "async"
        | "await" | "try" | "macro" | "union" => format!("{}_", string),
        _ => string.to_string(),
    }
}

/// Word-wrap `string` to 80 columns and emit it as a series of
/// `#[doc = "…"]` attributes.
///
/// We can't just use `///` line comments — the codegen produces a
/// [`TokenStream`] which then goes through `prettyplease`, and at that
/// level the doc comments have to be the desugared `#[doc = "…"]` form.
pub fn string_to_doc_comment(string: &str) -> TokenStream {
    let wrapped = fill(string, 80);
    let lines = wrapped.lines();

    let mut tokens = TokenStream::new();
    for line in lines {
        // Leading space inside the string mimics the `/// …` convention.
        let line = format!(" {}", line);
        tokens.extend(quote! {
            #[doc = #line]
        });
    }

    tokens
}

/// Translate a [`DataType`] runtime variant into the corresponding
/// compile-time marker path.
///
/// e.g. [`DataType::Float`] → `crate::data_type::Float`. Used wherever the
/// generated code needs to spell out a static type marker as a token
/// stream.
pub fn data_type_enum_to_data_type_marker(data_type: &DataType) -> TokenStream {
    match data_type {
        DataType::Bool => quote! { crate::data_type::Bool },
        DataType::Float => quote! { crate::data_type::Float},
        DataType::String => quote! { crate::data_type::String},
        DataType::Int => quote! { crate::data_type::Int},
        DataType::UnsignedInt => quote! { crate::data_type::UnsignedInt},
        DataType::Long => quote! { crate::data_type::Long},
        DataType::StereoSample => quote! { crate::data_type::StereoSample},
        DataType::Complex => quote! { crate::data_type::Complex},
        DataType::TensorFloat => quote! { crate::data_type::TensorFloat},
        DataType::VectorFloat => quote! { crate::data_type::VectorFloat},
        DataType::VectorString => quote! { crate::data_type::VectorString},
        DataType::VectorBool => quote! { crate::data_type::VectorBool},
        DataType::VectorInt => quote! { crate::data_type::VectorInt},
        DataType::VectorStereoSample => quote! { crate::data_type::VectorStereoSample},
        DataType::VectorComplex => quote! { crate::data_type::VectorComplex},
        DataType::VectorVectorFloat => quote! { crate::data_type::VectorVectorFloat},
        DataType::VectorVectorString => quote! { crate::data_type::VectorVectorString},
        DataType::VectorVectorStereoSample => quote! { crate::data_type::VectorVectorStereoSample},
        DataType::VectorVectorComplex => quote! { crate::data_type::VectorVectorComplex},
        DataType::VectorMatrixFloat => quote! { crate::data_type::VectorMatrixFloat},
        DataType::MapVectorFloat => quote! { crate::data_type::MapVectorFloat},
        DataType::MapVectorString => quote! { crate::data_type::MapVectorString},
        DataType::MapVectorInt => quote! { crate::data_type::MapVectorInt},
        DataType::MapVectorComplex => quote! { crate::data_type::MapVectorComplex},
        DataType::MapFloat => quote! { crate::data_type::MapFloat},
        DataType::MatrixFloat => quote! { crate::data_type::MatrixFloat},
        DataType::Pool => quote! { crate::data_type::Pool},
    }
}
