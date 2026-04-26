//! Per-algorithm code generation.
//!
//! Given an [`Introspection`] description of one Essentia algorithm, this
//! module produces a single Rust source file containing:
//!
//! * A typed builder struct in the [`Initialized`] state, with one method
//!   per parameter and a `configure()` method to advance to [`Configured`].
//! * A second `impl` block on the [`Configured`] state with the
//!   `compute(...)` method.
//! * A result struct returned by `compute(...)` with one accessor per
//!   output.
//! * (When applicable) sealed marker enums for string parameters that are
//!   constrained to a fixed set of values.
//!
//! The output is a [`syn::File`] which is then unparsed via `prettyplease`
//! and written to disk.

use std::path::Path;

use convert_case::{Case, Casing};
use essentia_core::Introspection;
use quote::format_ident;
use regex::Regex;
use syn::parse_quote;

use crate::algorithm_generation::{
    common::string_to_doc_comment, compute_function::generate_compute_function,
    output_functions::generate_output_functions, parameter_functions::generate_parameter_functions,
};

mod common;
mod compute_function;
mod output_functions;
mod parameter_functions;

/// Outcome of generating a single algorithm — the on-disk locations of
/// what was written, used by the module-glue step to wire `mod.rs` files
/// up later.
pub struct GeneratedAlgorithm {
    /// `snake_case` name of the algorithm's own module (the file name
    /// without `.rs`), e.g. `"beat_tracker_degara"`.
    pub algorithm_module_name: String,
    /// `snake_case` name of the category sub-directory the algorithm
    /// lives in, e.g. `"rhythm"`.
    pub category_module_name: String,
}

/// Build the [`syn::File`] AST for one algorithm.
///
/// Mostly delegates to the four submodules:
///
/// * [`parameter_functions::generate_parameter_functions`] — one method per
///   parameter, plus any sealed-enum constraint code.
/// * [`compute_function::generate_compute_function`] — the `compute(...)`
///   method that takes inputs as positional args.
/// * [`output_functions::generate_output_functions`] — one accessor per
///   output on the result struct.
/// * [`common::string_to_doc_comment`] — wraps free-form text into
///   `#[doc = "…"]` attributes for the Rust doc system.
pub fn generate_algorithm_module(introspection: &Introspection) -> syn::File {
    // PascalCase is Rust convention for type names, so the C++ algorithm
    // name (already PascalCase in practice) is just normalised here.
    let algorithm_struct_name =
        format_ident!("{}", &introspection.name().trim().to_case(Case::Pascal));

    // Result struct is named "<Algo>Result". This is the type returned by
    // a successful `compute()` call.
    let algorithm_result_struct_name = format_ident!(
        "{}Result",
        &introspection.name().trim().to_case(Case::Pascal)
    );

    // The original Essentia name is preserved verbatim as a string literal
    // — it's what the C++ side identifies the algorithm by.
    let algorithm_name = introspection.name();
    let description = string_to_doc_comment(introspection.description());
    let parameter_result = generate_parameter_functions(introspection);
    let compute_function =
        generate_compute_function(algorithm_result_struct_name.clone(), introspection);
    let output_functions = generate_output_functions(introspection);

    let constraint_code = &parameter_result.constraint_code;
    let parameter_functions = &parameter_result.functions;

    // Assemble the final module. The shape is:
    //
    //   <constraint enums for string-OneOf parameters, if any>
    //
    //   /// <doc>
    //   pub struct Algo<'a, State = Initialized> { algorithm: ... }
    //
    //   impl<'a> Algo<'a, Initialized> {
    //       fn parameter_n(self, value: …) -> Self { … } // one per parameter
    //       fn configure(self) -> Result<Algo<'a, Configured>, …> { … }
    //   }
    //
    //   impl<'a> Algo<'a, Configured> {
    //       fn compute(&mut self, input1: …, …) -> Result<AlgoResult<…>, …>
    //   }
    //
    //   impl<'a> CreateAlgorithm<'a> for Algo<'a, Initialized> { … }
    //
    //   pub struct AlgoResult<'a, 'r> { … }
    //   impl AlgoResult { fn output_n(&self) -> DataContainer<…> { … } } // per output
    parse_quote! {
        #constraint_code

        #description
        #[allow(dead_code)]
        pub struct #algorithm_struct_name<'a, State = crate::Initialized> {
            algorithm: essentia_core::algorithm::Algorithm<'a, State>
        }

        impl <'a> #algorithm_struct_name<'a, crate::Initialized> {
            #(#parameter_functions)*

            /// Configure the algorithm with the set parameters
            ///
            /// Returns a configured algorithm ready for computation.
            pub fn configure(self) -> Result<#algorithm_struct_name<'a, crate::Configured>, crate::algorithm::ConfigurationError> {
                Ok(#algorithm_struct_name {
                    algorithm: self.algorithm.configure().map_err(|e| match e {
                        essentia_core::algorithm::ConfigurationError::Internal(exception) => {
                            crate::algorithm::ConfigurationError::Internal(exception)
                        }
                    })?,
                })
            }
        }

        impl <'a> #algorithm_struct_name<'a, crate::Configured> {
            #compute_function
        }

        impl<'a> crate::algorithm::CreateAlgorithm<'a> for #algorithm_struct_name<'a, crate::Initialized> {
            fn create(essentia: &'a crate::Essentia) -> Self {
                let algorithm = match essentia.inner.create_algorithm(#algorithm_name) {
                    Ok(algorithm) => algorithm,
                    Err(essentia_core::CreateAlgorithmError::AlgorithmNotFound { name }) => {
                        panic!("Algorithm '{}' not found in Essentia", name);
                    }
                };

                Self { algorithm }
            }
        }

        #[allow(dead_code)]
        pub struct #algorithm_result_struct_name<'algorithm, 'result> {
            compute_result: essentia_core::algorithm::ComputeResult<'algorithm, 'result>
        }

        impl <'algorithm, 'result> #algorithm_result_struct_name<'algorithm, 'result> {
            #(#output_functions)*
        }
    }
}

/// Generate one algorithm and write the resulting Rust source to disk.
///
/// The output path follows the pattern
/// `<out_dir>/<category>/<algorithm>.rs`. Both `<category>` and
/// `<algorithm>` are derived from the introspection metadata via
/// `convert_case::Case::Snake`. Non-word characters in the category name
/// are collapsed to whitespace before snake-casing — Essentia's category
/// labels include things like `"Standard algorithms/Spectral"` that
/// otherwise wouldn't fit Rust module names.
pub fn generate_algorithm_module_file(
    introspection: &Introspection,
    out_dir: &Path,
) -> std::io::Result<GeneratedAlgorithm> {
    let algorithm_module_name = introspection.name().trim().to_case(Case::Snake);
    let category_module_name = Regex::new(r"\W+")
        .unwrap()
        .replace_all(introspection.category().trim(), " ")
        .trim()
        .to_case(Case::Snake);

    let category_module_directory_path = out_dir.join(&category_module_name);
    let algorithm_module_file_path =
        category_module_directory_path.join(format!("{}.rs", &algorithm_module_name));

    std::fs::create_dir_all(&category_module_directory_path)?;

    let syntax_tree = generate_algorithm_module(introspection);
    // `prettyplease` re-formats the AST into idiomatic Rust source. We
    // could write the unformatted output, but the diffs and panic
    // messages are easier to read this way.
    let formatted = prettyplease::unparse(&syntax_tree);
    std::fs::write(&algorithm_module_file_path, formatted)?;

    Ok(GeneratedAlgorithm {
        algorithm_module_name,
        category_module_name,
    })
}
