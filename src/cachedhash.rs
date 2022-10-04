use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

use crate::atomic::AtomicOptionNonZeroU64;

#[derive(Debug)]
pub struct CachedHash<T: Eq + Hash, BH: BuildHasher = BuildHasherDefault<DefaultHasher>> {
    value: T,
    hash: AtomicOptionNonZeroU64,
    build_hasher: BH,
}

impl<T: Eq + Hash> CachedHash<T> {
    pub fn new(value: T) -> Self {
        CachedHash::<T>::new_with_hasher(value)
    }
}

impl<T: Eq + Hash, H: Hasher + Default> CachedHash<T, BuildHasherDefault<H>> {
    pub fn new_with_hasher(value: T) -> Self {
        CachedHash::<T, BuildHasherDefault<H>>::new_with_build_hasher(value, Default::default())
    }
}

impl<T: Eq + Hash, BH: BuildHasher> CachedHash<T, BH> {
    pub fn new_with_build_hasher(value: T, build_hasher: BH) -> Self {
        CachedHash {
            value,
            hash: AtomicOptionNonZeroU64::new_none(),
            build_hasher,
        }
    }

    #[inline]
    pub fn invalidate_hash(this: &mut Self) {
        this.hash.set(None);
    }

    #[inline]
    #[must_use]
    pub fn take_value(this: Self) -> T {
        this.value
    }

    #[inline]
    #[must_use]
    pub fn get(this: &Self) -> &T {
        &this.value
    }

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
        CachedHash {
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
