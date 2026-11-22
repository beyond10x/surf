//! The one integration-test binary of `surf-agent-tasks`: the readers of `tasks.txt` and
//! `status.txt`, the fixture files, the judge, the rule of the report, the scripted lane over
//! the programs of `SURF_BIN_DIR` (`tasks::every_task`) and the model lane's driver run with a
//! stub model.

#![forbid(unsafe_code)]

mod fixtures;
mod format;
mod judge;
mod model_driver;
mod status;
mod tasks;
