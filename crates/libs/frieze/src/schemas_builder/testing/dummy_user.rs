use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Plain two-property struct schema registered as `User`, used as the
/// baseline root and as the target of
/// [`DummyProfile`](super::DummyProfile)'s reference.
pub(crate) struct DummyUser;

impl Schema for DummyUser {
    fn name() -> String {
        "User".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "User",
            vec![
                Property::new("id", PropertyType::Int64, Presence::Required).unwrap(),
                Property::new("name", PropertyType::String, Presence::Required).unwrap(),
            ],
        )
        .unwrap()
    }
}
impl Register for DummyUser {}
impl IsRegistrable for DummyUser {}
