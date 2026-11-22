//! One `surf-agent` call of the scripted agent against stand-ins: what is kept of its output is
//! capped. No surfd, no fixture server.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use surf_agent_tasks::agent::{Agent, OUTPUT_CAP, Step};
use surf_agent_tasks::process::RuntimeDir;

#[test]
fn a_call_whose_output_passes_the_cap_fails_naming_it() {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let bin = tmp.join(format!("calls-cap-{}", std::process::id()));
    let _ = fs::remove_dir_all(&bin);
    fs::create_dir_all(&bin).unwrap();
    // 100 bytes on standard output, then a wait the cap must not sit out.
    fs::write(
        bin.join("surf-agent"),
        "#!/bin/sh\nprintf '%0100d' 0\nexec /bin/sleep 30\n",
    )
    .unwrap();
    fs::set_permissions(bin.join("surf-agent"), fs::Permissions::from_mode(0o755)).unwrap();
    let dir = RuntimeDir::new(&tmp);
    let agent = Agent::new(bin.clone(), dir.runtime())
        .with_call_bound(Duration::from_secs(5))
        .with_output_cap(64);
    let started = std::time::Instant::now();
    let run = agent.run(
        &[Step::Open],
        "http://127.0.0.1:9/article/article.html",
        &mut || {},
    );
    let took = started.elapsed();
    let left = dir.processes();
    drop(dir);
    let _ = fs::remove_dir_all(&bin);
    assert_eq!(
        run.failure.map(|failure| failure.reason()),
        Some("output over 64 bytes on open".to_string())
    );
    assert!(took < Duration::from_secs(3), "the call took {took:?}");
    assert!(left.is_empty(), "processes of the call left: {left:?}");
    assert_eq!(OUTPUT_CAP, 1 << 20);
}
