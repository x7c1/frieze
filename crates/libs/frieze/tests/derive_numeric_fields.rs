use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Numbers {
    a_i32: i32,
    b_i64: i64,
    c_u32: u32,
    d_u64: u64,
    e_f32: f32,
    f_f64: f64,
}

#[test]
fn numeric_fields_render_with_correct_type_format_minimum() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Numbers>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @"
    Numbers:
      type: object
      required:
        - a_i32
        - b_i64
        - c_u32
        - d_u64
        - e_f32
        - f_f64
      properties:
        a_i32:
          type: integer
          format: int32
        b_i64:
          type: integer
          format: int64
        c_u32:
          type: integer
          format: int32
          minimum: 0
        d_u64:
          type: integer
          format: int64
          minimum: 0
        e_f32:
          type: number
          format: float
        f_f64:
          type: number
          format: double
    ");
}
