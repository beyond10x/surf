//! The one integration-test binary of `surf-agent-tasks`: the readers of `tasks.txt` and
//! `status.txt`, the fixture files, the judge, the rule of the report and its checks, the
//! scripted lane over the programs of `SURF_BIN_DIR` (`tasks::every_task`) and the model lane's
//! driver run with a stub model.

#![forbid(unsafe_code)]

mod adversary;
mod adversary2;
mod archive_task;
mod bounded;
mod calls;
mod check;
mod check_rule;
mod fixtures;
mod format;
mod https_row;
mod judge;
mod model_driver;
mod status;
mod tasks;
