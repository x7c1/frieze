use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Registered under the reserved primitive name `DateTime`. The struct
/// is deliberately **not** behind `#[cfg(feature = "chrono04")]`: the
/// reservation comes from `primitive_property_type_for`, which the
/// boundary consults regardless of the feature, so a build without
/// `chrono04` must reject the name just as firmly.
pub(crate) struct DummyDateTime;

impl Schema for DummyDateTime {
    fn name() -> String {
        "DateTime".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "DateTime",
            vec![Property::new("value", PropertyType::String, Presence::Required).unwrap()],
        )
        .unwrap()
    }
}
impl Register for DummyDateTime {}
impl IsRegistrable for DummyDateTime {}
