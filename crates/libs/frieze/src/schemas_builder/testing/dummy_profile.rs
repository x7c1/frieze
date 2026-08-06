use frieze_model::{Presence, Property, PropertyType, SchemaName};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// References `User` from a property without registering it, so adding
/// this root alone leaves the reference dangling.
pub(crate) struct DummyProfile;

impl Schema for DummyProfile {
    fn name() -> String {
        "Profile".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "Profile",
            vec![Property::new(
                "user",
                PropertyType::Reference(SchemaName::new("User").unwrap()),
                Presence::Required,
            )
            .unwrap()],
        )
        .unwrap()
    }
}
impl Register for DummyProfile {}
impl IsRegistrable for DummyProfile {}
