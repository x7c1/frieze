# Owned wrappers (`Box<T>`, `Rc<T>`, `Arc<T>`)

`std::boxed::Box`, `std::rc::Rc`, and `std::sync::Arc` are treated as
**transparent** owned wrappers with respect to the schema:

- `<Box<User> as Schema>::name()` returns `"User"`.
- `<Box<User> as Schema>::schema()` returns the same schema as
  `<User as Schema>::schema()`.
- The same delegation applies to `Rc<T>` and `Arc<T>`, and composes:
  `<Box<Box<User>> as Schema>::name() == "User"`.

This matches what serde produces on the wire: `Box<T>`, `Rc<T>`, and
`Arc<T>` all serialize as `T`'s wire form, so the schema must agree.

## Why transparency — recursive types

Recursive types in Rust require an indirection:

```rust
#[derive(Schema)]
struct Tree {
    value: i64,
    children: Vec<Box<Tree>>,   // Box<Tree> is required for sizedness
}
```

If `Box<Tree>` produced a separate schema entry instead of delegating
to `Tree`, every level of indirection would cascade into a new
synthetic schema name and the transitive-closure walk that resolves
`$ref` targets would never terminate. Transparent delegation gives a
self-referencing schema:

```yaml
Tree:
  type: object
  required: [value, children]
  properties:
    value: { type: integer, format: int64 }
    children:
      type: array
      items: { $ref: '#/components/schemas/Tree' }
```

`IsStructSchema` and `IsRegistrable` also propagate through the same
wrappers, so `Box<UserStruct>` is usable as the inner of an
internal-tagged enum variant and `Schemas::add::<Box<UserStruct>>()`
is equivalent to `Schemas::add::<UserStruct>()`.

## Scope: `Box` / `Rc` / `Arc` only

`Cell<T>`, `RefCell<T>`, `Mutex<T>`, and `RwLock<T>` are intentionally
**not** covered. They are interior-mutability primitives that rarely
appear in serialisable API shapes — a real REST handler typically
takes the lock, clones, then serializes, rather than serializing
through the lock guard. If the need arises later, the blanket impl
pattern in the `frieze` crate's `wrapper_impls` is the template to
follow.
