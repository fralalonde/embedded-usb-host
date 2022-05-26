/// Async mutex.

use core::cell::{UnsafeCell};
use core::mem::MaybeUninit;

use core::ops::{Deref, DerefMut};

use core::sync::atomic::Ordering::{Relaxed};
use atomic_polyfill::AtomicBool;
use spin::mutex::{SpinMutex, SpinMutexGuard};

// use crate::array_queue::ArrayQueue;

pub struct Local<T: Sized> {
    name: &'static str,
    init: AtomicBool,
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Sized + Send> Send for Local<T> {}

unsafe impl<T: Sized + Send> Sync for Local<T> {}

impl<T: Sized + Send> Local<T> {
    /// Create a new mutex with the given value.
    pub const fn uninit(name: &'static str) -> Self {
        Self {
            name,
            value: UnsafeCell::new(MaybeUninit::uninit()),
            init: AtomicBool::new(false),
        }
    }

    pub fn init_static(&self, value: T) -> &mut T {
        match self.init.compare_exchange(false, true, Relaxed, Relaxed) {
            Ok(false) => unsafe {
                let z = &mut (*self.value.get());
                *z.assume_init_mut() = value;
                return self.raw_mut();
            }
            err => {
                panic!("Mutex {:?} init twice: {:?}", self.name, err)
            }
        }
    }

    pub unsafe fn raw_mut(&self) -> &mut T {
        self.init_check();
        (&mut *(self.value.get())).assume_init_mut()
    }

    #[inline]
    fn init_check(&self) {
        if !self.init.load(Relaxed) { panic!("Local resource {} not initialized", self.name) } else {}
    }
}

impl<'a, T: Sized + Send> Deref for Local<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // self.init_check();
        unsafe { &*(self.value.get() as *const T) }
    }
}

impl<'a, T: Sized + Send> DerefMut for Local<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.init_check();
        unsafe { (&mut *(self.value.get())).assume_init_mut() }
    }
}

pub struct Shared<T: Sized> {
    name: &'static str,
    value: SpinMutex<MaybeUninit<T>>,
}

impl<T: Sized + Send> Shared<T> {
    /// Create a new mutex with the given value.
    pub const fn uninit(name: &'static str) -> Self {
        Self {
            name,
            value: SpinMutex::new(MaybeUninit::uninit()),
        }
    }
    pub fn init_static(&self, value: T) {
        // TODO init check
        unsafe { *self.value.lock().assume_init_mut() = value };
    }

    pub fn lock(&self) -> SharedGuard<'_, T> {
        SharedGuard {
            mutex: self.value.lock()
        }
    }
}

pub struct SharedGuard<'a, T: Sized, > {
    mutex: SpinMutexGuard<'a, MaybeUninit<T>>,
}

impl<'a, T: Sized, > Deref for SharedGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.mutex.assume_init_ref() }
    }
}

impl<'a, T: Sized, > DerefMut for SharedGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.mutex.assume_init_mut() }
    }
}

