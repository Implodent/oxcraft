//! NSFR - Not Safe For (safe) Rust (shillers)
//! Unsafe shit.

use std::cell::UnsafeCell;

pub struct CellerCell<T: ?Sized> {
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Sync> Sync for CellerCell<T> {}
unsafe impl<T: ?Sized + Send> Send for CellerCell<T> {}

impl<T: ?Sized> CellerCell<T> {
    #[inline]
    pub const fn new(value: T) -> Self
    where
        T: Sized,
    {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    #[inline]
    pub const fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.value.into_inner()
    }

    #[inline]
    pub const fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    #[inline]
    pub const fn get(&self) -> T
    where
        T: Copy,
    {
        unsafe { self.value.get().read() }
    }

    #[inline]
    pub fn replace(&mut self, value: T) -> T
    where
        T: Sized,
    {
        std::mem::replace(self.value.get_mut(), value)
    }
}
