use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

use crate::atomic::AtomicOptionNonZeroU64;

/// For a type `T`, [`CachedHash`] wraps `T` and implements [`Hash`] in a way that
/// caches `T`'s hash value. The first time the hash is computed, it is stored
/// and returned on subsequent calls. When the stored value is accessed mutably
/// the hash is invalidated and needs to be recomputed again. [`CachedHash`] implements
/// [`Deref`] and [`DerefMut`] so it can be used as a drop-in replacement for `T`.
///
/// In order for the hash to be invalidated correctly the stored type cannot use
/// interior mutability in a way that affects the hash. If this is the case, you
/// can use [`CachedHash::invalidate_hash`] to invalidate the hash manually.
///
/// By default the internal hash is computed using [`DefaultHasher`]. You can
/// change this by providing a custom [`Hasher`] to [`CachedHash::new_with_hasher`] or
/// even a custom [`BuildHasher`] to [`CachedHash::new_with_build_hasher`]. For most use
/// cases you should not need to do this.
///
/// Note that the hash of a value of type `T` and the same value wrapped in
/// [`CachedHash`] are generally different.
///
/// # Why is this useful?
///
/// Sometimes you have a type that is expensive to hash (for example because)
/// it is very large) but you need to store and move it between multiple
/// [`HashSet`](https://doc.rust-lang.org/std/collections/struct.HashSet.html)s
/// In this case you can wrap the type in [`CachedHash`] to cache the hash value
/// only once.
///
/// However, when the type is modified often [`CachedHash`] loses its advantage
/// as the hash will get invalidated on every modification. [`CachedHash`] also
/// needs to store the [`u64`] hash value which takes up some space.
///
/// You can run `cargo bench` to see some simple naive benchmarks comparing
/// a plaiin `HashSet` with a `HashSet` that stores values wrapped in [`CachedHash`].
#[derive(Debug)]
pub struct CachedHash<T: Eq + Hash, BH: BuildHasher = BuildHasherDefault<DefaultHasher>> {
    value: T,
    hash: AtomicOptionNonZeroU64,
    build_hasher: BH,
}

impl<T: Eq + Hash> CachedHash<T> {
    /// Creates a new [`CachedHash`] with the given value using [`DefaultHasher`].
    ///
    /// Note that the [`BuildHasher`] stored in the structure is a zero-sized type
    /// that is both [`Send`] and [`Sync`] so it will not affect the [`Send`] and [`Sync`]
    /// properties of [`CachedHash`] nor its size.
    pub fn new(value: T) -> Self {
        Self::new_with_hasher(value)
    }
}

impl<T: Eq + Hash, H: Hasher + Default> CachedHash<T, BuildHasherDefault<H>> {
    /// Creates a new [`CachedHash`] with the given value using a provided hasher type implementing [`Default`].
    ///
    /// Note that the [`BuildHasher`] stored in the structure is a zero-sized type
    /// that is both [`Send`] and [`Sync`] so it will not affect the [`Send`] and [`Sync`]
    /// properties of [`CachedHash`] nor its size.
    pub fn new_with_hasher(value: T) -> Self {
        Self::new_with_build_hasher(value, BuildHasherDefault::default())
    }
}

impl<T: Eq + Hash, BH: BuildHasher> CachedHash<T, BH> {
    /// Creates a new [`CachedHash`] with the given value and [`BuildHasher`].
    ///
    /// Note that `build_hasher` is stored in the structure and as such it can
    /// cause the type to stop being [`Send`] and [`Sync`] if the hasher is not.
    /// It can also increase the size of the structure.
    pub const fn new_with_build_hasher(value: T, build_hasher: BH) -> Self {
        Self {
            value,
            hash: AtomicOptionNonZeroU64::new_none(),
            build_hasher,
        }
    }

    /// Explicitly invalidates the cached hash. This should not be necessary
    /// in most cases as the hash will be automatically invalidated when
    /// the value is accessed mutably. However, if the value uses interior
    /// mutability in a way that affects the hash, you will need to call
    /// this function manually whenever the hash might have changed.
    #[inline]
    pub fn invalidate_hash(this: &mut Self) {
        this.hash.set(None);
    }

    /// Destructs the [`CachedHash`] and returns the stored value.
    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // false positive, `this` might get dropped
    pub fn take_value(this: Self) -> T {
        this.value
    }

    /// Explicitly returns an immutable reference to the stored value.
    ///
    /// Most of the time this will not be necessary as [`CachedHash`]
    /// implements [`Deref`] so autoderef rules will automatically dereference
    /// it to the stored value.
    #[inline]
    #[must_use]
    pub const fn get(this: &Self) -> &T {
        &this.value
    }

    /// Explicitly returns a mutable reference to the stored value and
    /// invalidates the cached hash.
    ///
    /// Most of the time this will not be necessary as [`CachedHash`] implements
    /// [`DerefMut`] so autoderef rules will automatically dereference it to the
    /// stored value. (Such dereference still invalidates the stored hash.)
    #[inline]
    #[must_use]
    pub fn get_mut(this: &mut Self) -> &mut T {
        Self::invalidate_hash(this);
        &mut this.value
    }
}

impl<T: Eq + Hash, BH: BuildHasher> PartialEq for CachedHash<T, BH> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq + Hash, BH: BuildHasher> Eq for CachedHash<T, BH> {}

impl<T: Eq + Hash, BH: BuildHasher> Hash for CachedHash<T, BH> {
    fn hash<H2: Hasher>(&self, state: &mut H2) {
        if let Some(hash) = self.hash.get_raw() {
            state.write_u64(hash);
        } else {
            let mut hasher = self.build_hasher.build_hasher();
            self.value.hash(&mut hasher);
            // MaybeHash can only store non-zero values so we create a small collision by bumping up hash 0 to 1.
            let hash = NonZeroU64::new(hasher.finish()).unwrap_or(NonZeroU64::new(1).unwrap());
            self.hash.set(Some(hash));
            state.write_u64(hash.into());
        }
    }
}

impl<T: Eq + Hash, BH: BuildHasher> AsMut<T> for CachedHash<T, BH> {
    fn as_mut(&mut self) -> &mut T {
        Self::get_mut(self)
    }
}

impl<T: Eq + Hash, BH: BuildHasher> AsRef<T> for CachedHash<T, BH> {
    fn as_ref(&self) -> &T {
        Self::get(self)
    }
}

impl<T: Eq + Hash, BH: BuildHasher> BorrowMut<T> for CachedHash<T, BH> {
    fn borrow_mut(&mut self) -> &mut T {
        Self::get_mut(self)
    }
}

impl<T: Eq + Hash, BH: BuildHasher> Borrow<T> for CachedHash<T, BH> {
    fn borrow(&self) -> &T {
        Self::get(self)
    }
}

impl<T: Eq + Hash, BH: BuildHasher> Deref for CachedHash<T, BH> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Self::get(self)
    }
}

impl<T: Eq + Hash, BH: BuildHasher> DerefMut for CachedHash<T, BH> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::get_mut(self)
    }
}

impl<T: Eq + Hash, H: Hasher + Default> From<T> for CachedHash<T, BuildHasherDefault<H>> {
    fn from(value: T) -> Self {
        Self::new_with_hasher(value)
    }
}

impl<T: Eq + Hash + Clone, BH: BuildHasher + Clone> Clone for CachedHash<T, BH> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            hash: self.hash.clone(),
            build_hasher: self.build_hasher.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::AtomicBool;

    use super::*;

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        calculate_hash_with_hasher::<T, DefaultHasher>(t)
    }

    fn calculate_hash_with_hasher<T: Hash, H: Hasher + Default>(t: &T) -> u64 {
        let mut s = H::default();
        t.hash(&mut s);
        s.finish()
    }

    #[test]
    fn hash_same_after_noop_mut_borrow() {
        let mut foo = super::CachedHash::new("foo".to_string());
        let hash = calculate_hash(&foo);
        let _ = CachedHash::get_mut(&mut foo);
        assert_eq!(hash, calculate_hash(&foo));
    }

    #[test]
    fn hash_different_after_modification() {
        let mut foo = super::CachedHash::new("foo".to_string());
        let hash = calculate_hash(&foo);
        foo.push('a');
        // The first three lines are there to make sure the test doesn't start failing
        // if the standard library hashing changes to create a collision in the test case.
        // This way it would only make this test useless.
        assert!(
            calculate_hash(&"foo".to_string()) == 0 || // unlikely case when 0 gets internally converted to 1
            calculate_hash(&"fooa".to_string()) == 0 || // ditto
            calculate_hash(&"foo".to_string()) == calculate_hash(&"fooa".to_string()) || // unlikely hash collision
            hash != calculate_hash(&foo)
        );
    }

    #[test]
    fn hash_same_after_invalidation() {
        let mut foo = super::CachedHash::new("foo".to_string());
        let hash = calculate_hash(&foo);
        CachedHash::invalidate_hash(&mut foo);
        assert_eq!(hash, calculate_hash(&foo));
    }

    #[test]
    fn hash_same_after_clone() {
        let foo = super::CachedHash::new("foo".to_string());
        let hash = calculate_hash(&foo);
        let foo2 = foo.clone();
        assert_eq!(hash, calculate_hash(&foo2));
    }

    #[test]
    fn hash_same_consecutive() {
        let foo = super::CachedHash::new("foo".to_string());
        let hash = calculate_hash(&foo);
        assert_eq!(hash, calculate_hash(&foo));
    }

    #[test]
    fn invalide_invalidates() {
        let mut foo = super::CachedHash::new("foo".to_string());
        assert!(foo.hash.get().is_none());
        calculate_hash(&foo);
        assert!(foo.hash.get().is_some());
        CachedHash::invalidate_hash(&mut foo);
        assert!(foo.hash.get().is_none());
        calculate_hash(&foo);
        assert!(foo.hash.get().is_some());
    }

    #[test]
    fn mut_deref_invalidates() {
        let mut foo = super::CachedHash::new("foo".to_string());
        assert!(foo.hash.get().is_none());
        calculate_hash(&foo);
        assert!(foo.hash.get().is_some());
        foo.push('a');
        assert!(foo.hash.get().is_none());
        calculate_hash(&foo);
        assert!(foo.hash.get().is_some());
        let _ = foo.len();
        assert!(foo.hash.get().is_some());
    }

    #[test]
    fn hash_gets_cached() {
        struct YouOnlyHashOnce {
            hashed_once: AtomicBool,
        }
        impl Eq for YouOnlyHashOnce {}
        impl PartialEq for YouOnlyHashOnce {
            fn eq(&self, _other: &Self) -> bool {
                true
            }
        }
        impl Hash for YouOnlyHashOnce {
            fn hash<H: Hasher>(&self, _state: &mut H) {
                if self
                    .hashed_once
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    panic!("Hashing should only happen once");
                }
            }
        }

        let foo = super::CachedHash::new(YouOnlyHashOnce {
            hashed_once: AtomicBool::new(false),
        });
        calculate_hash(&foo);
        calculate_hash(&foo);
        calculate_hash(&foo);
    }

    #[test]
    fn take_value() {
        let foo = super::CachedHash::new("foo".to_string());
        assert_eq!(CachedHash::take_value(foo), "foo".to_string());
    }

    #[test]
    fn struct_is_small() {
        assert!(
            std::mem::size_of::<super::CachedHash<String>>()
                <= std::mem::size_of::<String>() + std::mem::size_of::<u64>()
        );
    }

    #[test]
    fn zero_hash() {
        use nohash_hasher::NoHashHasher;

        struct FixedHash<const H: u64>();
        impl<const H: u64> Eq for FixedHash<H> {}
        impl<const H: u64> PartialEq for FixedHash<H> {
            fn eq(&self, _other: &Self) -> bool {
                true
            }
        }
        impl<const H: u64> Hash for FixedHash<H> {
            fn hash<HS: Hasher>(&self, state: &mut HS) {
                state.write_u64(H);
            }
        }

        assert!(
            calculate_hash_with_hasher::<FixedHash<0>, NoHashHasher<u64>>(&FixedHash::<0>()) == 0
        );
        let foo: CachedHash<_, BuildHasherDefault<NoHashHasher<u64>>> =
            super::CachedHash::new_with_hasher(FixedHash::<0>());
        let _ = calculate_hash(&foo);
        assert!(foo.hash.get().is_some());
    }
}
