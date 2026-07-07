//! End-to-end fixture: a member whose root type references a type
//! from a sibling workspace member.

use frieze::Schema;
use ws_shared::Person;

/// A support ticket.
#[derive(Schema)]
pub struct Ticket {
    /// The ticket's numeric id.
    pub id: i64,
    pub subject: String,
    pub assignee: Person,
}
