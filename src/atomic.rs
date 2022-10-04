use std::{fmt::Debug, num::NonZeroU64, sync::atomic::AtomicU64};

/// Think of this as a `Option<NonZeroU64>` but atomic.
#[repr(transparent)]
#[allow(clippy::module_name_repetitions)]
pub struct AtomicOptionNonZeroU64(AtomicU64);

impl AtomicOptionNonZeroU64 {
    pub const fn new_none() -> Self {
        Self(AtomicU64::new(0))
    }

    #[allow(dead_code)]
    pub fn new_some(value: NonZeroU64) -> Self {
        Self(AtomicU64::new(value.into()))
    }

    #[inline]
    pub fn get(&self) -> Option<NonZeroU64> {
        let value = self.0.load(std::sync::atomic::Ordering::Relaxed);
        if value == 0 {
            None
        } else {
            Some(value.try_into().unwrap())
        }
    }

    #[inline]
    pub fn get_raw(&self) -> Option<u64> {
        let value = self.0.load(std::sync::atomic::Ordering::Relaxed);
        if value == 0 {
            None
        } else {
            Some(value)
        }
    }

    #[inline]
    pub fn set(&self, value: Option<NonZeroU64>) {
        let value = value.map_or(0, Into::into);
        self.0.store(value, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for AtomicOptionNonZeroU64 {
    fn default() -> Self {
        Self::new_none()
    }
}

impl Debug for AtomicOptionNonZeroU64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl Clone for AtomicOptionNonZeroU64 {
    fn clone(&self) -> Self {
        Self(self.0.load(std::sync::atomic::Ordering::Relaxed).into())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;

    #[test]
    fn test_atomic_option_non_zero_u64() {
        let atomic = AtomicOptionNonZeroU64::new_none();
        assert_eq!(atomic.get(), None);
        assert_eq!(atomic.get_raw(), None);
        atomic.set(Some(NonZeroU64::new(1).unwrap()));
        assert_eq!(atomic.get(), Some(NonZeroU64::new(1).unwrap()));
        assert_eq!(atomic.get_raw(), Some(1));
        atomic.set(None);
        assert_eq!(atomic.get(), None);
        assert_eq!(atomic.get_raw(), None);
        let atomic = AtomicOptionNonZeroU64::new_some(NonZeroU64::new(1).unwrap());
        assert_eq!(atomic.get(), Some(NonZeroU64::new(1).unwrap()));
        assert_eq!(atomic.get_raw(), Some(1));
    }
}
