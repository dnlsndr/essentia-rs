//! [`Essentia`] — refcounted handle to the global C++ Essentia runtime.
//!
//! Essentia's C++ side relies on global state: an algorithm registry, FFTW
//! plans, log facilities, and so on. That state is initialised by
//! `essentia::init()` and torn down by `essentia::shutdown()`. Calling either
//! at the wrong time, more than once, or never, leads to crashes or leaks.
//!
//! This module hides the lifecycle behind a refcounted handle:
//!
//! * Internally, [`EssentiaLifecycle`] is a one-shot RAII guard:
//!   constructing it calls `init`, dropping it calls `shutdown`.
//! * [`Essentia`] holds an `Arc<EssentiaLifecycle>`, so multiple Rust-side
//!   handles share the same C++-side initialisation.
//! * A process-global `Mutex<Weak<EssentiaLifecycle>>` glues the picture
//!   together. The very first `Essentia::new` upgrades a stale `Weak`
//!   into a fresh `Arc` (which calls `init`). Subsequent `new`s find the
//!   `Weak` still upgradable and reuse it. Once every `Essentia` is
//!   dropped, the `Arc` count hits zero, drop fires, and `shutdown` runs.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex, Weak},
};

use essentia_sys::ffi;
use once_cell::sync::Lazy;

use crate::{
    algorithm::{Algorithm, Initialized},
    essentia::error::CreateAlgorithmError,
};

/// Process-global slot holding a [`Weak`] reference to the live
/// [`EssentiaLifecycle`]. Behind a [`Mutex`] so that two threads racing in
/// [`Essentia::new`] cannot both call `init`.
static GLOBAL_LIFECYCLE: Lazy<Mutex<Weak<EssentiaLifecycle>>> =
    Lazy::new(|| Mutex::new(Weak::new()));

/// Process-global cache of every algorithm name registered with the C++
/// runtime, populated lazily on first access.
///
/// Computed once and shared, because:
/// 1. It is a non-trivial FFI call.
/// 2. The set is invariant for the duration of the process — once
///    Essentia has been initialised, no new algorithms appear or disappear.
static AVAILABLE_ALGORITHMS: Lazy<HashSet<String>> =
    Lazy::new(|| ffi::get_algorithm_names().into_iter().collect());

/// One-shot RAII guard around the C++ runtime's init/shutdown pair.
///
/// Constructing it calls `essentia::init()`. Dropping it calls
/// `essentia::shutdown()`. There is exactly one of these alive at a time
/// across the whole process, kept alive by the [`Arc`]s inside [`Essentia`]
/// handles.
struct EssentiaLifecycle {}

impl EssentiaLifecycle {
    fn new() -> Self {
        ffi::init_essentia();
        Self {}
    }
}

impl Drop for EssentiaLifecycle {
    fn drop(&mut self) {
        ffi::shutdown_essentia();
    }
}

/// User-facing handle to the C++ Essentia runtime.
///
/// Cheap to clone and to construct — every handle aliases the same global
/// runtime via the inner [`Arc`]. The runtime stays initialised for as
/// long as any handle lives and is torn down when the last one drops.
///
/// `Essentia` is also the factory for [`Algorithm`]s — see
/// [`Self::create_algorithm`].
pub struct Essentia {
    /// The reference to the live C++ runtime. The leading underscore is
    /// because this field exists purely to keep the lifecycle's [`Arc`]
    /// count above zero — it is never read directly.
    _lifecycle: Arc<EssentiaLifecycle>,
}

impl Default for Essentia {
    fn default() -> Self {
        Self::new()
    }
}

impl Essentia {
    /// Create (or join) the C++ Essentia runtime and return a handle to it.
    ///
    /// Behaviour:
    ///
    /// * If no other [`Essentia`] handle currently exists in the process,
    ///   this initialises the C++ runtime (calling `essentia::init()`).
    /// * If at least one other handle is alive, this returns a fresh alias
    ///   to the same underlying lifecycle — no FFI call is made.
    ///
    /// Thread-safe; the global slot is guarded by a [`Mutex`] so two
    /// threads cannot both call `init` concurrently.
    pub fn new() -> Self {
        let mut global_lifecycle = GLOBAL_LIFECYCLE
            .lock()
            .expect("Failed to acquire lifecycle lock");

        if let Some(existing_lifecycle) = global_lifecycle.upgrade() {
            return Self {
                _lifecycle: existing_lifecycle,
            };
        }

        let lifecycle = Arc::new(EssentiaLifecycle::new());
        *global_lifecycle = Arc::downgrade(&lifecycle);

        Self {
            _lifecycle: lifecycle,
        }
    }

    /// Iterator over every algorithm name registered with the C++ runtime.
    ///
    /// Cheap — backed by a [`Lazy`] [`HashSet`] populated once on first
    /// use.
    pub fn available_algorithms(&self) -> impl Iterator<Item = &str> {
        AVAILABLE_ALGORITHMS.iter().map(|s| s.as_str())
    }

    /// Construct a new algorithm by name.
    ///
    /// The returned [`Algorithm`] is in the [`Initialized`] state, ready to
    /// receive parameter values. It borrows from `self` so the runtime
    /// cannot be torn down while the algorithm is still in use.
    ///
    /// Returns [`CreateAlgorithmError::AlgorithmNotFound`] if no algorithm
    /// with that name is registered. (See [`Self::available_algorithms`]
    /// for the full list.)
    pub fn create_algorithm<'a>(
        &'a self,
        algorithm_name: &str,
    ) -> Result<Algorithm<'a, Initialized>, CreateAlgorithmError> {
        if !AVAILABLE_ALGORITHMS.contains(algorithm_name) {
            return Err(CreateAlgorithmError::AlgorithmNotFound {
                name: algorithm_name.to_string(),
            });
        }

        // The membership check above guarantees this call cannot fail for
        // "algorithm not found" reasons; any error here would be a bug in
        // the bindings.
        let algorithm_bridge = ffi::create_algorithm_bridge(algorithm_name).expect(&format!(
            "failed to get algorithm '{}' after validation",
            algorithm_name
        ));

        Ok(Algorithm::new(algorithm_bridge))
    }
}

impl Clone for Essentia {
    /// Clone the handle. Cheap — only bumps the [`Arc`] refcount; no FFI
    /// call.
    fn clone(&self) -> Self {
        Self {
            _lifecycle: Arc::clone(&self._lifecycle),
        }
    }
}
