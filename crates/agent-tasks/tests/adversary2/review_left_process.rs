//! A case of `sleeping` that the two tests beside it do not hold: a `sleep` that still runs at
//! the first read and ends inside the bound is not left. Only a wait answers that; one read of
//! the state, which counts a zombie as ended too, takes the process for one left.

use super::*;

#[test]
fn a_process_that_ends_during_the_wait_is_not_left() {
    let mut child = Spawned(
        Command::new("/bin/sleep")
            .arg("0.5")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("/bin/sleep starts"),
    );
    let pid = child.0.id();
    let pids = tmp().join(format!(
        "adversary2-a_process_that_ends_during_the_wait_is_not_left-{}",
        std::process::id()
    ));
    fs::write(&pids, format!("{pid}\n")).unwrap();
    let left = sleeping(&pids, END_BOUND);
    let _ = child.0.wait();
    assert_eq!(
        left,
        Vec::<u32>::new(),
        "sleeping took a process that ended within its bound for one left"
    );
}
