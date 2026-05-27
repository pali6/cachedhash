use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;

use crate::atomic::nonzero_hash;

/// A borrowed-form lookup key for collections keyed on [`CachedHash<T, BH>`](crate::CachedHash).
///
/// `CachedHashKey<T, BH>` is a `#[repr(transparent)]` wrapper around `T` whose
/// [`Hash`] and [`Eq`] implementations are equivalent to those of
/// [`CachedHash<T, BH>`](crate::CachedHash). This lets you look up entries in a
/// `HashSet<CachedHash<T, BH>>` or `HashMap<CachedHash<T, BH>, V>` using a `&T`
/// without first allocating/wrapping it into a `CachedHash`.
///
/// Construct one with [`CachedHashKey::from_ref`], which performs a zero-cost
/// reference cast from `&T` to `&CachedHashKey<T, BH>`.
///
/// # Example
///
/// ```
/// use std::collections::HashSet;
/// use cachedhash::{CachedHash, CachedHashKey};
///
/// let mut set: HashSet<CachedHash<String>> = HashSet::new();
/// set.insert(CachedHash::new("foo".to_string()));
///
/// let needle = "foo".to_string();
/// assert!(set.contains(CachedHashKey::from_ref(&needle)));
/// ```
///
/// # Restrictions
///
/// The [`Borrow<CachedHashKey<T, BH>>`](std::borrow::Borrow) impl on
/// [`CachedHash`](crate::CachedHash) is only provided when `BH` is
/// [`BuildHasherDefault<H>`].
#[repr(transparent)]
pub struct CachedHashKey<T: ?Sized, BH = BuildHasherDefault<DefaultHasher>> {
    _phantom: PhantomData<fn() -> BH>,
    value: T,
}

impl<T: ?Sized, BH> CachedHashKey<T, BH> {
    /// Reinterpret a `&T` as a `&CachedHashKey<T, BH>`. Zero-cost operation.
    #[inline]
    #[must_use]
    pub const fn from_ref(value: &T) -> &Self {
        // SAFETY: CachedHashKey is repr(transparent) over T. The only
        // non-1-ZST field is value; _phantom is a 1-ZST. Therefore
        // CachedHashKey<T, BH> has identical layout, size, alignment, and ABI
        // to T, and &T is interchangeable with &CachedHashKey<T, BH>.
        #[allow(unsafe_code)]
        unsafe {
            &*(std::ptr::from_ref::<T>(value) as *const Self)
        }
    }

    /// Returns a reference to the wrapped value.
    #[inline]
    #[must_use]
    pub const fn get(&self) -> &T {
        &self.value
    }
}

impl<T: PartialEq + ?Sized, BH> PartialEq for CachedHashKey<T, BH> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq + ?Sized, BH> Eq for CachedHashKey<T, BH> {}

impl<T: Hash + ?Sized, BH: BuildHasher + Default> Hash for CachedHashKey<T, BH> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // has to mirror CachedHash's Hash impl to satify the Borrow restriction on Hash
        let hash = nonzero_hash(&BH::default(), &self.value);
        state.write_u64(hash.into());
    }
}

impl<T: std::fmt::Debug + ?Sized, BH> std::fmt::Debug for CachedHashKey<T, BH> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CachedHashKey").field(&&self.value).finish()
    }
}

impl<T: PartialOrd + ?Sized, BH> PartialOrd for CachedHashKey<T, BH> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord + ?Sized, BH> Ord for CachedHashKey<T, BH> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: ?Sized, BH> Deref for CachedHashKey<T, BH> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized, BH> AsRef<T> for CachedHashKey<T, BH> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::CachedHash;

    #[test]
    fn lookup_in_hashset() {
        let mut set: HashSet<CachedHash<String>> = HashSet::new();
        set.insert(CachedHash::new("foo".to_string()));
        set.insert(CachedHash::new("bar".to_string()));

        let needle = "foo".to_string();
        assert!(set.contains(CachedHashKey::from_ref(&needle)));
        let missing = "baz".to_string();
        assert!(!set.contains(CachedHashKey::from_ref(&missing)));
    }

    #[test]
    fn lookup_in_hashmap() {
        let mut map: HashMap<CachedHash<String>, i32> = HashMap::new();
        map.insert(CachedHash::new("foo".to_string()), 1);
        map.insert(CachedHash::new("bar".to_string()), 2);

        let needle = "foo".to_string();
        assert_eq!(map.get(CachedHashKey::from_ref(&needle)), Some(&1));
        let missing = "baz".to_string();
        assert_eq!(map.get(CachedHashKey::from_ref(&missing)), None);
    }

    #[test]
    fn remove_from_hashset() {
        let mut set: HashSet<CachedHash<String>> = HashSet::new();
        set.insert(CachedHash::new("foo".to_string()));
        let needle = "foo".to_string();
        assert!(set.remove(CachedHashKey::from_ref(&needle)));
        assert!(set.is_empty());
    }

    #[test]
    fn issue_2_failing_example_works() {
        // originally from issue #2: Used to panic because CachedHash implemented Borrow<T> with a non-equivalent Hash.
        let s = "foo";
        let cached = CachedHash::new(s);
        let mut set = HashSet::new();
        set.insert(cached.clone());
        assert!(set.contains(&cached));
        assert!(set.contains(CachedHashKey::from_ref(&s)));
    }

    #[test]
    fn key_is_same_size_as_t() {
        assert_eq!(
            std::mem::size_of::<CachedHashKey<String>>(),
            std::mem::size_of::<String>()
        );
        assert_eq!(
            std::mem::size_of::<CachedHashKey<u64>>(),
            std::mem::size_of::<u64>()
        );
    }

    #[test]
    fn hash_equivalent_to_cachedhash() {
        use std::hash::{Hash, Hasher};

        fn hash_one<T: Hash>(t: &T) -> u64 {
            let mut s = DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }

        let value = "hello".to_string();
        let cached = CachedHash::new(value.clone());
        assert_eq!(
            hash_one(&cached),
            hash_one(CachedHashKey::<String>::from_ref(&value))
        );
    }

    #[test]
    fn deref_and_as_ref() {
        fn takes_str(s: impl AsRef<str>) -> usize {
            s.as_ref().len()
        }

        let value = "hello".to_string();
        let key: &CachedHashKey<String> = CachedHashKey::from_ref(&value);

        assert_eq!(key.len(), 5);
        assert!(key.starts_with("he"));

        let s: &String = key.as_ref();
        assert_eq!(takes_str(s), 5);
    }
}
