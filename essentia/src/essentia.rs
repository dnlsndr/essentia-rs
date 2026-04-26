use crate::algorithm::CreateAlgorithm;

/// User-facing handle to the global Essentia C++ runtime.
///
/// Essentia is a C++ library whose internals (algorithm registry, factories,
/// etc.) live in a single global state. That state has to be **initialised
/// once** before any algorithm runs and **torn down once** when the program is
/// done with it.
///
/// `Essentia` hides that lifecycle behind a reference-counted handle:
///
/// * The first call to [`Essentia::new`] anywhere in the process triggers
///   `essentia::init()` on the C++ side.
/// * Each subsequent `Essentia::new` (or [`Clone`] of an existing handle) just
///   bumps the refcount.
/// * When the last `Essentia` handle is dropped, `essentia::shutdown()` runs.
///
/// Because of that, you can freely create as many `Essentia` values as you
/// like — they are all aliases of the same underlying runtime — but you must
/// keep at least one alive for the duration of any algorithm call.
///
/// In addition to lifecycle management, `Essentia` is the factory that produces
/// algorithm builders: see [`Essentia::create`].
///
/// # Example
///
/// ```ignore
/// use essentia::Essentia;
///
/// let essentia = Essentia::new();
/// // pass `&essentia` around or clone it; algorithms borrow from it
/// ```
pub struct Essentia {
    /// The actual reference-counted lifecycle, held in `essentia_core` so that
    /// both the codegen build dependency and this user-facing crate share a
    /// single global state.
    pub(crate) inner: essentia_core::Essentia,
}

impl Default for Essentia {
    fn default() -> Self {
        Self::new()
    }
}

impl Essentia {
    /// Create (or join) the Essentia runtime and return a handle to it.
    ///
    /// Cheap — only the very first call in the process actually initialises
    /// the C++ library. Subsequent calls share the same global state via
    /// reference counting.
    pub fn new() -> Self {
        Self {
            inner: essentia_core::Essentia::new(),
        }
    }

    /// Construct a new builder for the algorithm type `T`.
    ///
    /// `T` is one of the auto-generated algorithm structs under
    /// [`crate::algorithm`] (e.g. `algorithm::rhythm::BeatTrackerDegara`,
    /// `algorithm::spectral::Mfcc`, …). The returned value starts in the
    /// [`Initialized`](crate::Initialized) typestate, exposes a
    /// `parameter_name(value)` method per Essentia parameter, and is advanced
    /// to the [`Configured`](crate::Configured) typestate by calling
    /// `.configure()`.
    ///
    /// The returned builder borrows from `self`, so the `Essentia` handle must
    /// outlive every algorithm created from it. (This is what prevents the
    /// global runtime from being torn down while an algorithm is still in
    /// use.)
    pub fn create<'a, T: CreateAlgorithm<'a>>(&'a self) -> T {
        T::create(self)
    }
}
