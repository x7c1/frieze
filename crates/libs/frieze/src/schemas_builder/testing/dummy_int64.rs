use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Registered under the reserved primitive name `Int64`, which the
/// boundary conversion inlines as the scalar shape at every
/// reference position. Registering it must be rejected instead of
/// leaving the entry silently unreferenced.
pub(crate) struct DummyInt64;

impl Schema for DummyInt64 {
    fn name() -> String {
        "Int64".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "Int64",
            vec![Property::new("value", PropertyType::Int64, Presence::Required).unwrap()],
        )
        .unwrap()
    }
}
impl Register for DummyInt64 {}
impl IsRegistrable for DummyInt64 {}
