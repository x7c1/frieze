//! End-to-end fixture: shared types used by the `ws-api` member.

use frieze::Schema;

/// A person a ticket can be assigned to.
#[derive(Schema)]
pub struct Person {
    pub name: String,
    pub email: Option<String>,
}
