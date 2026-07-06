//! End-to-end fixture: derives compile with the `inventory` feature
//! off, but submit no registrations.

use frieze::Schema;

/// A value that would be collectable if `inventory` were enabled.
#[derive(Schema)]
pub struct Unreachable {
    pub id: i64,
}
