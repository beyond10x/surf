//! A case of `child_left` that no test of this file holds: a child that still runs at the first
//! read and ends inside the bound is not left. Only a wait answers that; one read of the state
//! takes the child for one left.

use super::*;

#[test]
fn a_child_that_ends_during_the_wait_is_not_left() {
    let state = state("review-ends-during-the-wait");
    let mut child = Command::new("/bin/sleep")
        .arg("0.5")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("/bin/sleep starts");
    fs::write(state.join("child"), format!("{}\n", child.id())).unwrap();
    let left = child_left(&state);
    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(&state);
    assert_eq!(
        left, None,
        "child_left took a child that ended within its bound for one left"
    );
}
