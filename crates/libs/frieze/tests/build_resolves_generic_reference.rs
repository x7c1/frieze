//! `Schemas::build()` resolves a `$ref` to a generic-instantiation
//! schema (`Container<User>` → `User_Container`) when the instantiation
//! has been explicitly registered, the same way it resolves any other
//! cross-schema reference.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    container: Container<User>,
}

#[test]
fn build_resolves_reference_to_generic_instance() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Profile>()
        .add::<Container<User>>()
        .add::<User>()
        .build()
        .expect("schemas build resolves a reference to a registered generic instance");

    // `Profile.container` resolves to the `User_Container` entry, which
    // in turn resolves to `User`. The transitive closure terminates.
    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Profile:
          type: object
          required:
          - container
          properties:
            container:
              $ref: '#/components/schemas/User_Container'
        User:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
        User_Container:
          type: object
          required:
          - value
          properties:
            value:
              $ref: '#/components/schemas/User'
    ");
}
