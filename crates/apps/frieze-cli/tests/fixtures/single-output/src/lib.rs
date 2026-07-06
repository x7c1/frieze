//! End-to-end fixture: one root type with a nested reference.

use frieze::Schema;

/// A registered user of the system.
#[derive(Schema)]
pub struct User {
    /// The user's numeric id.
    pub id: i64,
    pub name: String,
    pub profile: Profile,
    pub tags: Vec<String>,
}

/// Public profile data attached to a user.
#[derive(Schema)]
pub struct Profile {
    pub display_name: String,
    pub bio: Option<String>,
}
