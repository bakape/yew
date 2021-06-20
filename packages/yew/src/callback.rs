//! This module contains data types for interacting with `Scope`s.
//!
//! ## Relevant examples
//! - [Counter](https://github.com/yewstack/yew/tree/master/examples/counter)
//! - [Timer](https://github.com/yewstack/yew/tree/master/examples/timer)

/// Universal callback wrapper for function pointers. Note that closures that do not capture
/// variables are automatically compatible with function pointers.
///
/// Function pointers are uses instead of closures to enable cheap listener equality comparison.
/// This drastically reduces the cost patching listeners during VTag patching. Rust does not provide
/// any means to compare closures for equality.
///
/// <aside class="warning">
/// Use callbacks carefully, because if you call one from the `update` loop
/// of a `Component` (even from JS) it will delay a message until next.
/// Callbacks should be used from JS callbacks or `setTimeout` calls.
/// </aside>
#[derive(Copy, Eq, Debug)]
pub struct Callback<IN>(fn(IN));

impl<IN> From<fn(IN)> for Callback<IN> {
    fn from(func: fn(IN)) -> Self {
        Callback(func)
    }
}

impl<IN> Clone for Callback<IN> {
    fn clone(&self) -> Self {
        Self::new(self.0)
    }
}

impl<IN> PartialEq for Callback<IN> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<IN> Callback<IN> {
    /// Create new callback from function pointer.
    /// Note that closures that do not capture variables are automatically compatible with function
    /// pointers.
    pub fn new(func: fn(IN)) -> Self {
        Callback(func)
    }

    /// This method calls the callback's function.
    pub fn emit(&self, value: IN) {
        self.0(value);
    }

    /// Creates a "no-op" callback which can be used when it is not suitable to use an
    /// `Option<Callback>`.
    pub fn noop() -> Self {
        Self::new(|_| {})
    }
}

impl<IN> Default for Callback<IN> {
    fn default() -> Self {
        Self::noop()
    }
}
