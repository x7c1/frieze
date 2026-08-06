use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Registered under the reserved primitive name `Uuid`. The struct
/// is deliberately **not** behind `#[cfg(feature = "uuid1")]`: the
/// reservation comes from `primitive_property_type_for`, which the
/// boundary consults regardless of the feature, so a build without
/// `uuid1` must reject the name just as firmly.
pub(crate) struct DummyUuid;

impl Schema for DummyUuid {
    fn name() -> String {
        "Uuid".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "Uuid",
            vec![Property::new("value", PropertyType::String, Presence::Required).unwrap()],
        )
        .unwrap()
    }
}
impl Register for DummyUuid {}
impl IsRegistrable for DummyUuid {}
