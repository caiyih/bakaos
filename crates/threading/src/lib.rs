#![feature(noop_waker)]
#![feature(future_join)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

mod futures;

pub use futures::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_on() {
        let fut = async { 42 };

        assert_eq!(block_on!(fut), 42);

        let fut1 = async { 42 };
        let fut2 = async { 24 };

        assert_eq!(block_on!(fut1, fut2), (42, 24));
    }
}
