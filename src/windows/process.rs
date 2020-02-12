use std::fs::File;
use std::process::{Command, ExitStatus};

use errors::*;

pub struct PtyProcess {

}

impl PtyProcess {
    pub fn new(_command: Command) -> Result<Self> {
        unimplemented!()
    }
    pub fn get_file_handle(&self) -> File {
        unimplemented!()
    }
    pub fn set_kill_timeout(&mut self, _timeout_ms: Option<u64>) {
        unimplemented!()
    }
    pub fn exit_status(&self) -> Option<ExitStatus> {
        unimplemented!()
    }
}