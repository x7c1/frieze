//! End-to-end fixture: one schema set rendered as OAS 3.0 JSON and as
//! OAS 3.1 YAML.

use frieze::Schema;

/// An order placed in the shop.
#[derive(Schema)]
pub struct Order {
    pub id: i64,
    /// May hold no value — the field whose encoding differs between
    /// the OAS 3.0 and 3.1 shapes.
    pub note: Option<String>,
}
