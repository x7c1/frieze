use frieze_model::{Presence, Property, PropertyType};

use crate::register::{IsRegistrable, Register};
use crate::schema::Schema;

/// Registered under the reserved primitive name `Date`. Like
/// [`super::DummyDateTime`], it stays outside
/// `#[cfg(feature = "chrono04")]` so the unconditional reservation is
/// exercised in a build without the feature.
pub(crate) struct DummyDate;

impl Schema for DummyDate {
    fn name() -> String {
        "Date".to_string()
    }
    fn schema() -> frieze_model::Schema {
        frieze_model::Schema::new_object(
            "Date",
            vec![Property::new("value", PropertyType::String, Presence::Required).unwrap()],
        )
        .unwrap()
    }
}
impl Register for DummyDate {}
impl IsRegistrable for DummyDate {}
