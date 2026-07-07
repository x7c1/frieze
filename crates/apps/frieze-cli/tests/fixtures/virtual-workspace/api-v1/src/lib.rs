//! End-to-end fixture: the frieze-configured member of a virtual
//! workspace.

use frieze::Schema;

/// A widget in the catalog.
#[derive(Schema)]
pub struct Widget {
    /// The widget's numeric id.
    pub id: i64,
    pub label: String,
}
