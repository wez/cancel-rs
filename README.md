# Cancellation Tokens in Rust

[![Build Status](https://travis-ci.org/wez/cancel-rs.svg?branch=master)](https://travis-ci.org/wez/cancel-rs)

[Documentation](https://docs.rs/cancel)

This crate provides a simple token that can be used to co-operatively
manage cancellation of an operation.  Setting a deadline/timeout is
also supported.

## Usage

### Explicit interactive cancellation

This one is a little awkward to show a complete working sketch, but the concept
is simple: pass the token to a long running operation and have it check for
cancellation every so often.

Then you can wire up a button click or CTRL-C so that it calls
`token.cancel()`.

Note that the implementation of `Token::cancel()` is a simple atomic operation
and is async signal safe.

```rust
use cancel::{Canceled, Token};

fn do_something(token: Arc<Token>) -> Result<bool, Canceled> {
  while !done {
    token.check_cancel()?;

    // process more stuff here
  }

  Ok(true)
}

fn cancel_button_clicked(token: Arc<Token>) {
  token.cancel();
}
```

### Simple timeout management

In this scenario the token has been configured with a deadline.  The deadline
is co-operatively checked by the `do_something` function when it calls
`check_cancel`.

```rust
use cancel::{Canceled, Token};
use std::time::Duration;

fn do_something(token: &Token) -> Result<bool, Canceled> {
  while !done {
    token.check_cancel()?;

    // process more stuff here
  }

  Ok(true)
}

fn start_something() -> Result<bool, Canceled> {
  let token = Token::with_duration(Duration::new(10, 0));
  do_something(&token)
}
```
