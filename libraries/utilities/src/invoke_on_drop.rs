use core::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

pub struct InvokeOnDrop<T, F: FnOnce(T)> {
    func: Option<F>,
    val: ManuallyDrop<T>,
}

impl<F: FnOnce(())> InvokeOnDrop<(), F> {
    pub fn new(func: F) -> Self {
        Self::transform((), func)
    }
}

impl<T, F: FnOnce(T)> InvokeOnDrop<T, F> {
    pub fn transform(val: T, func: F) -> Self {
        InvokeOnDrop {
            func: Some(func),
            val: ManuallyDrop::new(val),
        }
    }
}

impl<T, F: FnOnce(T)> Drop for InvokeOnDrop<T, F> {
    fn drop(&mut self) {
        if let Some(func) = self.func.take() {
            func(unsafe { ManuallyDrop::take(&mut self.val) });
        } else {
            panic!("InvokeOnDrop dropped without func, perhaps it was already dropped?");
        }
    }
}

impl<T: Copy, F: FnOnce(T)> InvokeOnDrop<T, F> {
    pub fn as_val(&self) -> T {
        *self.val
    }
}

impl<F: FnOnce(T), T> Deref for InvokeOnDrop<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<T, F: FnOnce(T)> DerefMut for InvokeOnDrop<T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn test_drop_invoked() {
        let mut called = false;

        {
            let _ = InvokeOnDrop::new(|_| {
                called = true;
            });
        }

        assert!(called);
    }

    #[test]
    fn test_drop_not_invoked() {
        let flag = Arc::new(Mutex::new(false));

        // Do NOT use discard to avoid drop call inlined
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
}
