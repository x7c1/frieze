//! End-to-end fixture: two outputs generated from one schema set.

use frieze::Schema;

/// A pet listed in the store.
#[derive(Schema)]
pub struct Pet {
    pub id: i64,
    pub name: String,
    pub owner: Owner,
}

/// The registered owner of a pet.
#[derive(Schema)]
pub struct Owner {
    pub id: i64,
    pub email: Option<String>,
}
