use core::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

/// A RAII guard that automatically invokes a closure when dropped.
///
/// `InvokeOnDrop` provides a way to ensure that cleanup code is executed when a value
/// goes out of scope, similar to destructors in C++ or `defer` statements in other languages.
/// This is particularly useful in systems programming where manual resource management
/// is required.
///
/// The guard holds both a value of type `T` and a closure of type `F` that takes `T`
/// as its parameter. When the guard is dropped, the closure is called with the value.
///
/// # Type Parameters
///
/// * `T` - The type of the value to be managed
/// * `F` - The type of the cleanup closure, must implement `FnOnce(T)`
///
/// # Examples
///
/// ## Basic cleanup
///
/// ```
/// use utilities::InvokeOnDrop;
/// use std::sync::{Arc, Mutex};
///
/// let flag = Arc::new(Mutex::new(false));
/// let flag_clone = flag.clone();
///
/// {
///     let _guard = InvokeOnDrop::new(|_| {
///         *flag_clone.lock().unwrap() = true;
///     });
///     // flag is still false here
/// }
/// // flag is now true - cleanup was executed
/// ```
///
/// ## Managing a resource with automatic cleanup
///
/// ```
/// use utilities::InvokeOnDrop;
///
/// fn acquire_resource() -> i32 { 42 }
/// fn release_resource(id: i32) { /* cleanup logic */ }
///
/// let resource_id = acquire_resource();
/// let _guard = InvokeOnDrop::transform(resource_id, |id| {
///     release_resource(id);
/// });
/// // resource will be automatically released when guard goes out of scope
/// ```
#[must_use = "hold the guard in a local variable to delay the callback until scope exit"]
pub struct InvokeOnDrop<T, F: FnOnce(T)> {
    func: ManuallyDrop<F>,
    val: ManuallyDrop<T>,
}

impl<F: FnOnce(())> InvokeOnDrop<(), F> {
    /// Creates a new `InvokeOnDrop` guard with a unit value.
    ///
    /// This is a convenience method for when you only need to execute cleanup
    /// code without managing a specific value.
    ///
    /// # Parameters
    ///
    /// * `func` - The cleanup closure to invoke when the guard is dropped
    ///
    /// # Returns
    ///
    /// A new `InvokeOnDrop` guard that will execute `func` on drop
    ///
    /// # Examples
    ///
    /// ```
    /// use utilities::InvokeOnDrop;
    ///
    /// let _guard = InvokeOnDrop::new(|_| {
    ///     println!("Cleanup executed!");
    /// });
    /// // "Cleanup executed!" will be printed when guard goes out of scope
    /// ```
    #[inline]
    pub fn new(func: F) -> Self {
        Self::transform((), func)
    }
}

impl<T, F: FnOnce(T)> InvokeOnDrop<T, F> {
    /// Creates a new `InvokeOnDrop` guard with a specific value and cleanup function.
    ///
    /// This method allows you to associate a cleanup closure with a value. When the
    /// guard is dropped, the closure will be called with the value as its parameter.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to be managed by the guard
    /// * `func` - The cleanup closure that will receive the value when the guard is dropped
    ///
    /// # Returns
    ///
    /// A new `InvokeOnDrop` guard managing the value
    ///
    /// # Examples
    ///
    /// ```
    /// use utilities::InvokeOnDrop;
    ///
    /// let guard = InvokeOnDrop::transform(42, |value| {
    ///     println!("Cleaning up value: {}", value);
    /// });
    /// 
    /// assert_eq!(*guard, 42);
    /// // "Cleaning up value: 42" will be printed when guard goes out of scope
    /// ```
    #[inline]
    pub fn transform(val: T, func: F) -> Self {
        InvokeOnDrop {
            func: ManuallyDrop::new(func),
            val: ManuallyDrop::new(val),
        }
    }

    /// Deconstructs the `InvokeOnDrop` guard, returning the value and function without executing cleanup.
    /// 
    /// This method allows you to retrieve the managed value and cleanup function without
    /// triggering the automatic cleanup behavior. This is useful when you want to handle
    /// the cleanup manually or transfer ownership of the value elsewhere.
    ///
    /// # Returns
    ///
    /// A tuple containing the managed value and the cleanup function
    ///
    /// # Examples
    ///
    /// ```
    /// use utilities::InvokeOnDrop;
    ///
    /// let guard = InvokeOnDrop::transform(42, |value| {
    ///     println!("This won't be printed");
    /// });
    ///
    /// let (value, cleanup_fn) = guard.deconstruct();
    /// assert_eq!(value, 42);
    /// // cleanup_fn can be called manually if needed
    /// cleanup_fn(value); // Now "This won't be printed" is printed
    /// ```
    ///
    /// # Safety
    ///
    /// This method uses unsafe code to extract values from `ManuallyDrop` and prevent
    /// the destructor from running. The extracted values are guaranteed to be valid
    /// as long as the original guard was valid.
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

    /// Consume the `InvokeOnDrop` guard and drop the inner value immediately without executing cleanup.
    ///
    /// This method cancels the cleanup operation and immediately drops both the managed
    /// value and the cleanup function. This is useful when you want to abort the cleanup
    /// process entirely.
    ///
    /// # Examples
    ///
    /// ```
    /// use utilities::InvokeOnDrop;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let flag = Arc::new(Mutex::new(false));
    /// let flag_clone = flag.clone();
    ///
    /// let guard = InvokeOnDrop::new(|_| {
    ///     *flag_clone.lock().unwrap() = true;
    /// });
    ///
    /// guard.cancel(); // Cancel the cleanup
    /// 
    /// // flag remains false because cleanup was cancelled
    /// assert!(!*flag.lock().unwrap());
    /// ```
    ///
    /// # Note
    ///
    /// The value is dropped before the function to ensure proper cleanup order.
    /// This method uses `deconstruct` internally to avoid issues during stack unwinding.
    pub fn cancel(self) {
        // use deconstruct to avoid drop being called during unwind
        let (val, func) = self.deconstruct();

        drop(val); // val should be dropped before func
        drop(func);
    }
}

/// Implementation of the `Drop` trait for `InvokeOnDrop`.
///
/// When an `InvokeOnDrop` instance is dropped, this implementation extracts both
/// the managed value and the cleanup function from their `ManuallyDrop` wrappers
/// and then invokes the cleanup function with the value.
///
/// # Safety
///
/// This implementation uses unsafe code to extract values from `ManuallyDrop`.
/// This is safe because:
/// - The values are only extracted once during drop
/// - The extracted values are immediately used and not accessed again
/// - No other code can access the guard after drop begins
impl<T, F: FnOnce(T)> Drop for InvokeOnDrop<T, F> {
    fn drop(&mut self) {
        let func = unsafe { ManuallyDrop::take(&mut self.func) };
        let val = unsafe { ManuallyDrop::take(&mut self.val) };

        func(val);
    }
}

impl<T: Copy, F: FnOnce(T)> InvokeOnDrop<T, F> {
    /// Returns a copy of the managed value.
    ///
    /// This method is only available when the managed type `T` implements `Copy`.
    /// It provides access to the value without consuming the guard.
    ///
    /// # Returns
    ///
    /// A copy of the managed value
    ///
    /// # Examples
    ///
    /// ```
    /// use utilities::InvokeOnDrop;
    ///
    /// let guard = InvokeOnDrop::transform(42, |_| {});
    /// let value_copy = guard.as_val();
    /// assert_eq!(value_copy, 42);
    /// // guard is still valid and will execute cleanup when dropped
    /// ```
    #[inline]
    pub fn as_val(&self) -> T {
        *self.val
    }
}

/// Implementation of `Deref` trait for `InvokeOnDrop`.
///
/// This allows the guard to be transparently used as if it were the managed value.
/// You can call methods on the managed value directly through the guard.
///
/// # Examples
///
/// ```
/// use utilities::InvokeOnDrop;
///
/// let guard = InvokeOnDrop::transform(String::from("hello"), |_| {});
/// 
/// // Can use string methods directly on the guard
/// assert_eq!(guard.len(), 5);
/// assert!(guard.contains("ell"));
/// ```
impl<T, F: FnOnce(T)> Deref for InvokeOnDrop<T, F> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

/// Implementation of `DerefMut` trait for `InvokeOnDrop`.
///
/// This allows mutable access to the managed value through the guard.
/// You can modify the managed value, and these changes will be visible
/// to the cleanup function when it's called.
///
/// # Examples
///
/// ```
/// use utilities::InvokeOnDrop;
///
/// let mut guard = InvokeOnDrop::transform(42, |final_value| {
///     println!("Final value: {}", final_value);
/// });
/// 
/// *guard = 100; // Modify the managed value
/// // When guard is dropped, cleanup will receive 100, not 42
/// ```
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
