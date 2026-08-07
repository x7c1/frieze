//! chrono field shapes under OAS 3.1, behind the opt-in `chrono04`
//! feature. `Option<DateTime<Utc>>` inlines to a nullable scalar, which
//! 3.1 encodes by folding `"null"` into the `type` sequence.
//!
//! `DateTime<Utc>` and `DateTime<FixedOffset>` sit side by side to pin
//! that the time zone stays out of the schema: both render the
//! identical `{type: string, format: date-time}` shape, because chrono's
//! serde default writes an RFC 3339 string either way. (`DateTime<Local>`
//! is the third supported time zone, covered by the unit tests instead:
//! naming it here would only repeat the same rendered shape.)
#![cfg(feature = "chrono04")]

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Booking {
    created_at: DateTime<Utc>,
    confirmed_at: DateTime<FixedOffset>,
    checkout_on: NaiveDate,
    cancelled_at: Option<DateTime<Utc>>,
    blackout_dates: Vec<NaiveDate>,
}

#[test]
fn chrono_fields_render_as_string_with_date_formats_under_oas_3_1() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Booking>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml_3_1(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Booking:
          type: object
          required:
          - created_at
          - confirmed_at
          - checkout_on
          - cancelled_at
          - blackout_dates
          properties:
            created_at:
              type: string
              format: date-time
            confirmed_at:
              type: string
              format: date-time
            checkout_on:
              type: string
              format: date
            cancelled_at:
              type:
              - string
              - 'null'
              format: date-time
            blackout_dates:
              type: array
              items:
                type: string
                format: date
    ");
}
