use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

use crate::atomic::AtomicOptionNonZeroU64;

#[derive(Debug)]
pub struct CachedHash<T: Eq + Hash, H: Hasher + Default = DefaultHasher> {
    value: T,
    hash: AtomicOptionNonZeroU64,
    _hasher_phantom: std::marker::PhantomData<H>,
}

impl<T: Eq + Hash> CachedHash<T> {
    pub fn new(value: T) -> Self {
        CachedHash::<T, DefaultHasher>::new_with_hasher(value)
    }
}

impl<T: Eq + Hash, H: Hasher + Default> CachedHash<T, H> {
    pub fn new_with_hasher(value: T) -> Self {
        CachedHash {
            value,
            hash: AtomicOptionNonZeroU64::new_none(),
            _hasher_phantom: std::marker::PhantomData,
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

impl<T: Eq + Hash, H: Hasher + Default> PartialEq for CachedHash<T, H> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq + Hash, H: Hasher + Default> Eq for CachedHash<T, H> {}

impl<T: Eq + Hash, H: Hasher + Default> Hash for CachedHash<T, H> {
    fn hash<H2: Hasher>(&self, state: &mut H2) {
        if let Some(hash) = self.hash.get_raw() {
            state.write_u64(hash);
        } else {
            let mut hasher = H::default();
            self.value.hash(&mut hasher);
            // MaybeHash can only store non-zero values so we create a small collision by bumping up hash 0 to 1.
            let hash = NonZeroU64::new(hasher.finish()).unwrap_or(NonZeroU64::new(1).unwrap());
            self.hash.set(Some(hash));
            state.write_u64(hash.into());
        }
    }
}

impl<T: Eq + Hash, H: Hasher + Default> Deref for CachedHash<T, H> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Self::get(self)
    }
}

impl<T: Eq + Hash, H: Hasher + Default> DerefMut for CachedHash<T, H> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::get_mut(self)
    }
}

impl<T: Eq + Hash + Clone, H: Hasher + Default> Clone for CachedHash<T, H> {
    fn clone(&self) -> Self {
        CachedHash {
            value: self.value.clone(),
            hash: self.hash.clone(),
            _hasher_phantom: std::marker::PhantomData,
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
}
