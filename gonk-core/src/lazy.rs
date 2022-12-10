//! Lazy Static
//!
//! There are no guarantees on safety when using `BasedCell`.
//!
//! It wouldn't be based otherwise would it?.
use std::{
    fmt::Debug,
    mem,
    ops::{Deref, DerefMut},
    sync::Once,
};

#[derive(Debug)]
#[repr(transparent)]
pub struct BasedCell<T: ?Sized> {
    pub value: T,
}

impl<T> BasedCell<T> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self { value }
    }
    pub fn replace(&self, val: T) -> T {
        #[allow(clippy::cast_ref_to_mut)]
        mem::replace(unsafe { &mut *(&self.value as *const T as *mut T) }, val)
    }
    #[inline(always)]
    pub const fn get(&self) -> *mut T {
        // We can just cast the pointer from `UnsafeCell<T>` to `T` because of
        // #[repr(transparent)]. This exploits libstd's special status, there is
        // no guarantee for user code that this will work in future versions of the compiler!
        self as *const BasedCell<T> as *const T as *mut T
    }
}

///If used on an immutable static, bad things will happen :)
pub struct Lazy<T, F = fn() -> T> {
    pub data: BasedCell<Option<T>>,
    function: F,
    once: Once,
}

impl<T, F> Lazy<T, F> {
    pub const fn new(f: F) -> Self {
        Self {
            data: BasedCell::new(None),
            function: f,
            once: Once::new(),
        }
    }
}

impl<T, F: Fn() -> T> Deref for Lazy<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.once.call_once(|| {
            let f = &self.function;
            let t = f();
            self.data.replace(Some(t));
        });

        self.data.value.as_ref().unwrap()
    }
}

impl<T, F: Fn() -> T> DerefMut for Lazy<T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.once.call_once(|| {
            let f = &self.function;
            let t = f();
            self.data.replace(Some(t));
        });

        self.data.value.as_mut().unwrap()
    }
}
