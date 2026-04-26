//! Build script for the user-facing `essentia` crate.
//!
//! This crate's algorithm tree is generated from C++ Essentia's runtime
//! introspection (see [`essentia_codegen`]). Rather than emit the tree to
//! `OUT_DIR` and `include!` it from there — which hides the generated
//! source from IDEs and is generally awkward to navigate — we write it
//! into a dedicated, deterministic path under this crate's `src/` tree:
//!
//! ```text
//! essentia/src/algorithm/generated/
//! ├── mod.rs                       (one `pub mod <category>;` per category)
//! ├── rhythm/
//! │   ├── mod.rs
//! │   ├── beat_tracker_degara.rs
//! │   └── …
//! └── …
//! ```
//!
//! The directory is gitignored — it is regenerated on every build — but
//! its absolute path is stable across `cargo clean`s, which means
//! `rust-analyzer` and any other Rust tooling can resolve it like an
//! ordinary `mod` declaration. The result: full jump-to-def and
//! autocomplete on every generated algorithm.
//!
//! If you want to regenerate the tree explicitly (without going through
//! `cargo build`), run:
//!
//! ```sh
//! cargo run -p essentia-codegen
//! ```

use std::path::PathBuf;

/// Path of the generated module tree, relative to this crate's manifest.
/// Kept in sync with the `mod generated;` declaration in
/// `src/algorithm/mod.rs`.
const GENERATED_SUBPATH: &str = "src/algorithm/generated";

fn main() -> std::io::Result<()> {
    // docs.rs builds without the native libraries that essentia-sys
    // requires, so the published documentation is generated with an
    // empty algorithm tree. We still need a valid `mod.rs` to keep
    // the rest of the crate compiling, so emit a stub.
    if std::env::var("DOCS_RS").is_ok() {
        println!("cargo:warning=Skipping codegen on docs.rs");
        return write_empty_stub();
    }

    println!("cargo:rerun-if-changed=build.rs");
    // Trigger a re-run if anything inside the codegen crate changes.
    // Without this, edits to `essentia_codegen` would not regenerate
    // the algorithm tree until the user `cargo clean`ed.
    println!("cargo:rerun-if-changed=../essentia_codegen/src");

    let target_dir = generated_dir();
    // Wipe and recreate the directory so removed algorithms (e.g. after
    // an Essentia upgrade) don't leave stale `.rs` files behind.
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
    }
    std::fs::create_dir_all(&target_dir)?;

    essentia_codegen::generate_code(&target_dir)?;

    Ok(())
}

/// Resolve `<manifest>/src/algorithm/generated` as an absolute path.
fn generated_dir() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is always set during build script execution");
    PathBuf::from(manifest).join(GENERATED_SUBPATH)
}

/// Write a placeholder `generated/mod.rs` so the crate still compiles
/// when codegen is intentionally skipped (currently only on docs.rs).
fn write_empty_stub() -> std::io::Result<()> {
    let target_dir = generated_dir();
    std::fs::create_dir_all(&target_dir)?;
    std::fs::write(
        target_dir.join("mod.rs"),
        "// Auto-generated stub — codegen was skipped.\n\
         // This file exists so the rest of the crate still compiles when\n\
         // the algorithm tree was not regenerated (e.g. on docs.rs).\n",
    )
}
