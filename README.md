# CachedHash

[![Build Status][build_img]][build_lnk] [![Crates.io][crates_img]][crates_lnk] [![Docs.rs][doc_img]][doc_lnk]

[build_img]: https://github.com/pali6/cachedhash/actions/workflows/build.yml/badge.svg

[build_lnk]: https://github.com/pali6/cachedhash/actions

[crates_img]: https://img.shields.io/crates/v/cachedhash.svg

[crates_lnk]: https://crates.io/crates/cachedhash

[doc_img]: https://docs.rs/cachedhash/badge.svg

[doc_lnk]: https://docs.rs/cachedhash

For a type `T`, `CachedHash<T>` wraps `T` and implements `Hash` in a way that
caches `T`'s hash value. This is useful when `T` is expensive to hash (for
example if it contains a large vector) and you need to hash it multiple times
with few modifications (for example by moving it between multiple `HashSet`s).

Stored hash is invalidated whenever the stored value is accessed mutably (via
`DerefMut`, `AsMut`, `BorrowMut` or explicitly via a provided associated function).
In order for the hash to be invalidated correctly the stored type cannot use
interior mutability in a way that affects the hash. If this is the case, you
can use `CachedHash::invalidate_hash` to invalidate the hash manually.

## License

Licensed under either of

 * Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
