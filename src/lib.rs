//! This crate provides a `Token` that can be used to co-operatively
//! signal when an operation should be canceled.
//!
//! ```rust
//! use cancel::{Canceled, Token};
//! use std::time::Duration;
//!
//! fn do_something(token: &Token) -> Result<bool, Canceled> {
//!   loop {
//!     token.check_cancel()?;
//!
//!     // process more stuff here
//!   }
//!
//!   Ok(true)
//! }
//!
//! fn start_something() -> Result<bool, Canceled> {
//!   let token = Token::with_duration(Duration::new(10, 0));
//!   do_something(&token)
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The Err value returned from `Token::check_cancel`.
/// It indicates that the `Token` was canceled and that the operation
/// should cease.
#[derive(Debug)]
pub struct Canceled {}

impl std::error::Error for Canceled {}
impl std::fmt::Display for Canceled {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Operation was Canceled")
    }
}

/// A cancellation token.
/// It tracks the state and holds an optional deadline for the operation.
/// To share `Token` across threads, wrap it in a `std::sync::Arc`.
#[derive(Debug, Default)]
pub struct Token {
    canceled: AtomicBool,
    deadline: Option<Instant>,
}

impl Token {
    /// Create a new Token with no deadline.  The token
    /// will be marked as canceled only once `Token::cancel`
    /// has been called.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a new Token with a deadline set to the current
    /// clock plus the specified duration.  The token will be
    /// marked as canceled either when `Token::cancel` is
    /// called, or when the operation calls either `Token::is_canceled`
    /// or `Token::check_cancel` and the current clock exceeds
    /// the computed deadline.
    pub fn with_duration(duration: Duration) -> Self {
        Self {
            canceled: AtomicBool::new(false),
            deadline: Some(Instant::now() + duration),
        }
    }

    /// Create a new Token with a deadline set to the specified
    /// instant.  The token will be marked as canceled either when
    /// `Token::cancel` is called, or when the operation calls
    /// either `Token::is_canceled` or `Token::check_cancel` and
    /// the current clock exceeds the specified deadline.
    pub fn with_deadline(deadline: Instant) -> Self {
        Self {
            canceled: AtomicBool::new(false),
            deadline: Some(deadline),
        }
    }

    /// Explicitly mark the token as being canceled.
    /// This method is async signal safe.
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
    }

    /// Check whether the token was canceled.
    /// This method is intended to be called by code that initiated
    /// (rather than performed) an operation to test whether that
    /// operation was successful.
    /// If you want to test for cancellation in the body of your
    /// processing code you should use either `Token::is_canceled`
    /// or `Token::check_cancel`.
    /// Using `Token::check_cancel` to propagate a `Result` value
    /// is often a cleaner design than using `Token::was_canceled`.
    pub fn was_canceled(&self) -> bool {
        self.canceled.load(Ordering::Acquire)
    }

    /// Test whether an ongoing operation should cease
    /// due to cancellation.
    /// If a deadline has been set, the current clock will be evaluated
    /// and compared against the deadline, setting the state to canceled
    /// if appropriate.
    /// Returns true if the operation has been canceled.
    pub fn is_canceled(&self) -> bool {
        if self.was_canceled() {
            true
        } else if let Some(deadline) = self.deadline.as_ref() {
            if Instant::now() > *deadline {
                self.cancel();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Test whether an ongoing operation should cease
    /// due to cancellation, propagating a `Canceled` error value
    /// if the operation has been canceled.
    /// If a deadline has been set, the current clock will be evaluated
    /// and compared against the deadline, setting the state to canceled
    /// if appropriate.
    pub fn check_cancel(&self) -> Result<(), Canceled> {
        if self.is_canceled() {
            Err(Canceled {})
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use failure::Fallible;
    use std::sync::Arc;

    #[test]
    fn it_works() {
        let token = Token::new();
        assert!(!token.was_canceled());
        token.cancel();
        assert!(token.was_canceled());
    }

    // Ensure that we work with the failure crate, but don't force our
    // users to require the failure crate
    fn check(token: &Token) -> Fallible<()> {
        token.check_cancel()?;
        Ok(())
    }

    #[test]
    fn err() {
        let token = Token::new();
        token.cancel();
        assert_eq!(true, token.check_cancel().is_err());
        assert_eq!(true, check(&token).is_err());
    }

    #[test]
    fn deadline() {
        let hard_deadline = Instant::now() + Duration::new(2, 0);
        let token = Token::with_duration(Duration::new(1, 0));
        loop {
            if token.is_canceled() {
                break;
            }

            assert!(Instant::now() < hard_deadline);
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    #[test]
    fn threads() {
        let token = Arc::new(Token::with_duration(Duration::new(1, 0)));
        let shared = Arc::clone(&token);
        let thr = std::thread::spawn(move || {
            while !shared.is_canceled() {
                std::thread::sleep(Duration::from_millis(200));
            }
            true
        });
        assert_eq!(true, thr.join().unwrap());
        assert_eq!(true, token.was_canceled());
    }
}
