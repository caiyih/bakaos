use core::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

#[must_use = "hold the guard in a local variable to delay the callback until scope exit"]
pub struct InvokeOnDrop<T, F: FnOnce(T)> {
    func: ManuallyDrop<F>,
    val: ManuallyDrop<T>,
}

impl<F: FnOnce(())> InvokeOnDrop<(), F> {
    #[inline]
    pub fn new(func: F) -> Self {
        Self::transform((), func)
    }
}

impl<T, F: FnOnce(T)> InvokeOnDrop<T, F> {
    #[inline]
    pub fn transform(val: T, func: F) -> Self {
        InvokeOnDrop {
            func: ManuallyDrop::new(func),
            val: ManuallyDrop::new(val),
        }
    }

    /// Deconstructs the `InvokeOnDrop`, cancelling the function
    pub fn deconstruct(mut self) -> (T, F) {
        let (val, func) = unsafe {
            (
                ManuallyDrop::take(&mut self.val),
                ManuallyDrop::take(&mut self.func),
            )
        };

        core::mem::forget(self);

        (val, func)
    }

    /// Consume the `InvokeOnDrop`, skip the callback, and drop the inner value immediately.
    pub fn cancel(self) {
        // use deconstruct to avoid drop being called during unwind
        let (val, func) = self.deconstruct();

        drop(val); // val should be dropped before func
        drop(func);
    }
}

impl<T, F: FnOnce(T)> Drop for InvokeOnDrop<T, F> {
    fn drop(&mut self) {
        let func = unsafe { ManuallyDrop::take(&mut self.func) };
        let val = unsafe { ManuallyDrop::take(&mut self.val) };

        func(val);
    }
}

impl<T: Copy, F: FnOnce(T)> InvokeOnDrop<T, F> {
    #[inline]
    pub fn as_val(&self) -> T {
        *self.val
    }
}

impl<T, F: FnOnce(T)> Deref for InvokeOnDrop<T, F> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<T, F: FnOnce(T)> DerefMut for InvokeOnDrop<T, F> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

#[cfg(test)]
mod tests {
    use core::hint::black_box;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn test_drop_invoked() {
        let flag = Arc::new(Mutex::new(false));

        {
            let _called = InvokeOnDrop::new(|_| {
                *flag.lock().unwrap() = true;
            });

            assert!(!*flag.lock().unwrap());
        }

        assert!(*flag.lock().unwrap());
    }

    #[test]
    fn test_drop_not_invoked() {
        let flag = Arc::new(Mutex::new(false));

        // Bind to a local so Drop doesn't run before the assertion
        let _unused = InvokeOnDrop::new(|_| {
            *flag.lock().unwrap() = true;
        });

        assert!(!*flag.lock().unwrap());
    }

    #[test]
    fn test_transform() {
        let i = InvokeOnDrop::transform(42, |i| {
            assert_eq!(i, 42);
        });

        assert_eq!(i.as_val(), 42);
        assert_eq!(*i, 42);
    }

    #[test]
    fn test_deref() {
        let mut i = InvokeOnDrop::transform(42, |i| {
            assert_eq!(i, 24);
        });

        assert_eq!(*i, 42);

        *i = 24;

        assert_eq!(*i, 24);
    }

    #[test]
    fn test_cancel_not_leak() {
        let x = Arc::new(());

        let cloned = x.clone();
        let guard = InvokeOnDrop::new(|_| {
            black_box(cloned);
        });

        // strong count should be 2
        assert_eq!(Arc::strong_count(&x), 2);

        guard.cancel();

        // strong count should be 1
        assert_eq!(Arc::strong_count(&x), 1);
    }
}
