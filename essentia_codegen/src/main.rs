//! Standalone codegen runner.
//!
//! Regenerates the per-algorithm Rust source files into a target directory
//! by querying the C++ Essentia runtime for its registered algorithms and
//! their introspection metadata.
//!
//! The build script of the user-facing `essentia` crate already invokes
//! the same library function automatically on every build, so most users
//! never need this binary. It exists for two cases:
//!
//! * Regenerating into a custom directory for inspection / diffing.
//! * Forcing a regeneration without a full `cargo build`, e.g. after
//!   upgrading Essentia in a Docker image.
//!
//! ## Usage
//!
//! ```sh
//! # Regenerate into the canonical location (essentia/src/algorithm/generated):
//! cargo run -p essentia-codegen
//!
//! # Regenerate into a custom path (useful for inspection):
//! cargo run -p essentia-codegen -- /tmp/essentia-algorithms
//! ```
//!
//! The target directory is wiped and recreated, so any stale files left
//! behind by an earlier Essentia version are removed.

use std::path::{Path, PathBuf};

/// Default target relative to the workspace root, kept in sync with the
/// `mod generated;` declaration in `essentia/src/algorithm/mod.rs`.
const DEFAULT_TARGET: &str = "essentia/src/algorithm/generated";

fn main() -> std::io::Result<()> {
    let target = parse_target_dir();

    if target.exists() {
        eprintln!("Removing existing tree at {}", target.display());
        std::fs::remove_dir_all(&target)?;
    }
    std::fs::create_dir_all(&target)?;

    eprintln!("Regenerating algorithm tree into {}", target.display());
    essentia_codegen::generate_code(&target)?;
    eprintln!("Done.");

    Ok(())
}

/// Take the first positional argument as the target directory, or fall
/// back to [`DEFAULT_TARGET`] resolved relative to the workspace root.
///
/// We resolve the workspace root from this crate's `CARGO_MANIFEST_DIR`
/// (`<workspace>/essentia_codegen`) so that `cargo run -p essentia-codegen`
/// behaves the same regardless of the user's current working directory.
fn parse_target_dir() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        return PathBuf::from(arg);
    }

    let codegen_manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is set when run via `cargo run`");
    let workspace_root = Path::new(&codegen_manifest)
        .parent()
        .expect("essentia_codegen always lives under the workspace root");
    workspace_root.join(DEFAULT_TARGET)
}
