//! `surf-view-foreign`: test tooling, not shipped. It stands for a process of somebody else on
//! the machine that looks like a session: `tests/fixture.rs` starts it through a symbolic link
//! named `surf-session`, with a session's id among its arguments, and checks that the fixture's
//! `kill` leaves it alive.
//!
//! It is not `tail -f /dev/null` started under that name, as it was: on some Linux images `tail`
//! is one name of a program that serves many utilities and chooses by the name it was started
//! under, so started as `surf-session` it exits at once. This program ignores its name and its
//! arguments, starts nothing, sets no timer and waits until it is killed.

#![forbid(unsafe_code)]

fn main() {
    loop {
        std::thread::park();
    }
}
