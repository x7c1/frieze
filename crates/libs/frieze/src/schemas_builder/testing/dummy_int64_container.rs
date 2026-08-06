use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Composed generic name that merely *starts* with a reserved name
/// (`Container<i64>` registers as `Int64_Container`). Only exact
/// matches are reserved, so this must keep building.
pub(crate) struct DummyInt64Container;

impl Schema for DummyInt64Container {
    fn name() -> String {
        "Int64_Container".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "Int64_Container",
            vec![Property::new("value", PropertyType::Int64, Presence::Required).unwrap()],
        )
        .unwrap()
    }
}
impl Register for DummyInt64Container {}
impl IsRegistrable for DummyInt64Container {}
