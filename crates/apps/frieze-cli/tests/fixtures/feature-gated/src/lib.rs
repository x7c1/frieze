//! End-to-end fixture: one always-present type and one behind a cargo
//! feature that is off by default — only the `features` declaration in
//! the frieze metadata turns it on during collection.

use frieze::Schema;

/// Always compiled into the crate.
#[derive(Schema)]
pub struct Base {
    pub id: i64,
}

/// Compiled — and therefore registered — only when the `extra`
/// feature is enabled.
#[cfg(feature = "extra")]
#[derive(Schema)]
pub struct Gated {
    pub note: String,
}
