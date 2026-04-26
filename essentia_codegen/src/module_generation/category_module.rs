//! Generation of per-category `mod.rs` files.
//!
//! Each category sub-directory (e.g. `<out_dir>/rhythm/`) gets a `mod.rs`
//! that declares each algorithm submodule and re-exports its public items
//! at the category level. The end result is that users can write either
//!
//! ```ignore
//! use essentia::algorithm::rhythm::beat_tracker_degara::BeatTrackerDegara;
//! // or
//! use essentia::algorithm::rhythm::BeatTrackerDegara;
//! ```
//!
//! Both work because each category `mod.rs` carries `pub use
//! <algo_module>::*;` for every algorithm in it.

use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_quote;

/// Write `<out_dir>/<category_name>/mod.rs` declaring every algorithm
/// module in `algorithm_module_names` and re-exporting their contents.
///
/// Algorithm names are sorted before writing, so the output is
/// deterministic across builds.
pub fn generate_category_module_file(
    out_dir: &Path,
    category_name: &str,
    algorithm_module_names: &[String],
) -> std::io::Result<()> {
    let module_file_path = out_dir.join(category_name).join("mod.rs");

    let mut sorted_algorithm_module_names = algorithm_module_names.to_vec();
    sorted_algorithm_module_names.sort();

    // `pub mod <algo>;` declarations, one per algorithm.
    let algorithm_module_declarations: Vec<TokenStream> = sorted_algorithm_module_names
        .iter()
        .map(|algorithm_module_name| {
            let algorithm_module_identifier = format_ident!("{}", algorithm_module_name);
            quote! {
                pub mod #algorithm_module_identifier;
            }
        })
        .collect();

    // `pub use <algo>::*;` re-exports so callers can omit the per-algo
    // module path.
    let algorithm_module_re_exports: Vec<TokenStream> = sorted_algorithm_module_names
        .iter()
        .map(|algorithm_module_name| {
            let algorithm_module_identifier = format_ident!("{}", algorithm_module_name);
            quote! {
                pub use #algorithm_module_identifier::*;
            }
        })
        .collect();

    let syntax_tree = parse_quote! {
        // Auto-generated file
        // This file contains all algorithm modules for the category `#category_name`

        #(#algorithm_module_declarations)*

        #(#algorithm_module_re_exports)*
    };

    let formatted = prettyplease::unparse(&syntax_tree);
    std::fs::write(module_file_path, formatted).unwrap();
    Ok(())
}
