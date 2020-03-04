use std::fs::File;
use std::process::ExitStatus;

use crate::errors::*;
// use crate::Command;

use super::{Command, PtyReader, PtyWriter};

pub struct PtyProcess {

}

impl PtyProcess {
    pub fn new(_command: Command) -> Result<Self> {
        unimplemented!()
    }
    // pub fn get_file_handle(&self) -> File {
    //     unimplemented!()
    // }
    pub fn get_io_handles(&self) -> Result<(PtyReader, PtyWriter)> {
        unimplemented!()
    }
    pub fn set_kill_timeout(&mut self, _timeout_ms: Option<u64>) {
        unimplemented!()
    }
    pub fn exit_status(&self) -> Option<ExitStatus> {
        unimplemented!()
    }
}