//! Hand-written [`Schema`](crate::Schema) / [`Register`](crate::Register)
//! fixtures shared by the unit tests of this module tree.
//!
//! Every type here leaves its `impl Register` block empty: the default
//! `register_into` pushes only `Self`, so these impls exercise the
//! non-recursive default path and let the tests assert the low-level
//! builder behaviour (silent dedup, conflict and reserved-name
//! reporting, unresolved-reference detection) without depending on the
//! derive.

mod dummy_int64;
pub(super) use dummy_int64::DummyInt64;

mod dummy_int64_container;
pub(super) use dummy_int64_container::DummyInt64Container;

mod dummy_namespaced_int64;
pub(super) use dummy_namespaced_int64::DummyNamespacedInt64;

mod dummy_profile;
pub(super) use dummy_profile::DummyProfile;

mod dummy_user;
pub(super) use dummy_user::DummyUser;

mod dummy_user_alt;
pub(super) use dummy_user_alt::DummyUserAlt;

mod dummy_uuid;
pub(super) use dummy_uuid::DummyUuid;
