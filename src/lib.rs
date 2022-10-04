//! For a type `T`, [`CachedHash<T>`](CachedHash) wraps `T` and
//! implements [`Hash`](https://doc.rust-lang.org/std/hash/trait.Hash.html) in a way that
//! caches `T`'s hash value. This is useful when `T` is expensive to hash (for
//! example if it contains a large vector) and you need to hash it multiple times
//! with few modifications (for example by moving it between multiple
//! [`HashSet`](https://doc.rust-lang.org/std/collections/struct.HashSet.html)s).
//!
//! Stored hash is invalidated whenever the stored value is accessed mutably (via
//! [`DerefMut`](https://doc.rust-lang.org/std/ops/trait.DerefMut.html),
//! [`AsMut`](https://doc.rust-lang.org/std/convert/trait.AsMut.html),
//! [`BorrowMut`](https://doc.rust-lang.org/std/borrow/trait.BorrowMut.html)
//! or explicitly via a provided [associated function](CachedHash::get_mut)).
//! In order for the hash to be invalidated correctly the stored type cannot use
//! interior mutability in a way that affects the hash. If this is the case, you
//! can use [`CachedHash::invalidate_hash`](CachedHash::invalidate_hash)
//!  to invalidate the hash manually.

mod atomic;
mod cachedhash;

pub use cachedhash::CachedHash;
