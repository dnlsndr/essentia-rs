//! Generation of output accessors on the per-algorithm `<Algo>Result`
//! struct.
//!
//! For each output declared in an algorithm's introspection, this module
//! produces a method
//!
//! ```ignore
//! pub fn output_name(&self) -> DataContainer<'result, crate::data_type::Foo> { … }
//! ```
//!
//! that returns a typed [`DataContainer`](essentia_core::DataContainer)
//! borrowing from the algorithm's internal output buffer. Users can then
//! call `.get()` (or `.try_get()`) on the container to read the value as a
//! Rust type via the conversion traits.
//!
//! As with `compute` and the parameter setters, runtime mismatches are
//! statically unreachable and so produce panics rather than `Result`s.

use convert_case::{Case, Casing};
use essentia_core::Introspection;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::algorithm_generation::common::{
    data_type_enum_to_data_type_marker, sanitize_identifier_string, string_to_doc_comment,
};

/// Emit one method per declared output, ready to be dropped into the
/// `<Algo>Result` impl block.
pub fn generate_output_functions(introspection: &Introspection) -> Vec<TokenStream> {
    introspection
        .outputs()
        .map(|output| {
            let method_name = format_ident!(
                "{}",
                &sanitize_identifier_string(&output.name().to_case(Case::Snake))
            );
            let output_name = output.name();
            let variant = data_type_enum_to_data_type_marker(&output.input_output_type());

            let doc_comment = string_to_doc_comment(&format!(
                "Get the `{}` output from the computation result.\n\n# Description\n\n{}",
                output_name,
                output.description()
            ));

            quote! {
                #doc_comment
                pub fn #method_name(&self) -> crate::DataContainer<'result, #variant> {
                    match self.compute_result.output(#output_name) {
                        Ok(output) => output,
                        Err(essentia_core::algorithm::OutputError::OutputNotFound { output }) => {
                            panic!("Output '{}' not found after validation", output);
                        }
                        Err(essentia_core::algorithm::OutputError::TypeMismatch { output, expected, actual }) => {
                            panic!("Type mismatch for output '{}': expected {:?}, found {:?}", output, expected, actual);
                        }
                    }
                }
            }
        })
        .collect()
}
