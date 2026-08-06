use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Same registration name as [`DummyUser`](super::DummyUser) ("User")
/// but with a different property set, used to exercise the same-name /
/// different-content path in [`crate::SchemasBuilder::push_unique`].
pub(crate) struct DummyUserAlt;

impl Schema for DummyUserAlt {
    fn name() -> String {
        "User".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "User",
            vec![
                Property::new("id", PropertyType::Int64, Presence::Required).unwrap(),
                // `DummyUser` has `name: String`; we substitute
                // `email: String` so the schemas share a name but
                // differ in content.
                Property::new("email", PropertyType::String, Presence::Required).unwrap(),
            ],
        )
        .unwrap()
    }
}
impl Register for DummyUserAlt {}
impl IsRegistrable for DummyUserAlt {}
