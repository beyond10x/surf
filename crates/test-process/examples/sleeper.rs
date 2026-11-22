//! A child of the tests of `surf-test-process`: it sleeps the milliseconds of its first argument
//! (10,000 when there is none), then exits 0. `sleeper 0` exits at once. A program of this
//! repository, so that the tests start no program of the machine.

#![forbid(unsafe_code)]

use std::time::Duration;

fn main() {
    let millis = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(10_000);
    std::thread::sleep(Duration::from_millis(millis));
}
