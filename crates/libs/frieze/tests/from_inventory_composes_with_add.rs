//! End-to-end: `from_inventory()` and `add::<T>()` compose into a
//! single chain.
//!
//! `add` here registers `Box<Profile>` — a transparent blanket-impl
//! root that piggy-backs on `Profile`'s schema. Because the inventory
//! channel already submitted `Profile` via its own derive, the
//! `add::<Box<Profile>>()` call's transitive walk hits the idempotent
//! guard at the top of `Profile::register_into` and silently merges
//! into the existing entry. The final schemas set has exactly one
//! `Profile` entry, demonstrating the dedup contract spans the
//! inventory channel as well as the explicit `add` channel.
//!
//! This test is only meaningful with `--features inventory`; the file
//! is gated accordingly.

#![cfg(feature = "inventory")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    nickname: String,
}

#[test]
fn from_inventory_composes_with_explicit_add() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        // `Box<Profile>` is a blanket-impl root over the already-
        // registered `Profile`. The `register_into` guard collapses
        // the second arrival into the existing entry — confirming
        // the two channels coexist without double-registering.
        .add::<Box<Profile>>()
        .build()
        .expect("inventory + add chain produces a closed schemas set");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Profile:
      type: object
      required:
        - nickname
      properties:
        nickname:
          type: string
    "##);
}
