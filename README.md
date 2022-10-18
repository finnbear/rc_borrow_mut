# rc_borrow_mut

This crate can be used to mutably borrow the contents of an `Rc`, if there are no other
strong references, despite any `Weak` references.

## ⚠ Warning ⚠

This crate is unsound, as it [can result in use-after-free in safe code](https://github.com/rust-lang/libs-team/issues/112#issuecomment-1282274231).

```rust
let mut rc = Rc::new("asdf".to_string());
std::mem::forget(Rc::borrow_mut(&mut rc));
drop(rc.clone());
println!("use after free: {rc:?}");
```

## Example

```rust
use std::rc::Rc;
use rc_borrow_mut::RcBorrowMut;

let mut rc = Rc::new(0);
let weak = Rc::downgrade(&Rc::clone(&rc));

// Wont panic, since there are no other strong references.
let mut mutable = Rc::borrow_mut(&mut rc);

*mutable += 1;

// Weak references don't work during mutation.
assert!(weak.upgrade().is_none());

*mutable += 1;
drop(mutable);

// Weak references start working again after we are done mutating.
assert_eq!(*weak.upgrade().unwrap(), 2);
```

## Other Warnings

- Requires `nightly` Rust
- Uses `unsafe`
- Depends on unspecified implementation details of `Rc`
- Not rigorously verified

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.