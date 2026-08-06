use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Namespaced name whose bare segment is a reserved name
/// (`#[frieze(namespace)] mod v1 { struct Int64 }`). A
/// namespaced name always carries a dot, so it can never equal a
/// reserved bare name and stays acceptable.
pub(crate) struct DummyNamespacedInt64;

impl Schema for DummyNamespacedInt64 {
    fn name() -> String {
        "v1.Int64".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "v1.Int64",
            vec![Property::new("value", PropertyType::Int64, Presence::Required).unwrap()],
        )
        .unwrap()
    }
}
impl Register for DummyNamespacedInt64 {}
impl IsRegistrable for DummyNamespacedInt64 {}
