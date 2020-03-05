use std::fs::File;
use std::process::ExitStatus;

use winapi::um::winnt::HANDLE;

use crate::errors::*;
// use crate::Command;

use super::{Command, PtyReader, PtyWriter};

pub struct PtyProcess {
    io: Option<(PtyReader, PtyWriter)>,
    pty: HANDLE,
    proc: HANDLE,
}

impl PtyProcess {
    pub fn new(command: Command) -> Result<Self> {
        command.spawn_pty().chain_err(||"Could not spawn PtyProcess")
    }
    pub(crate) fn init(pty_read: PtyReader, pty_write: PtyWriter, pty: HANDLE, proc: HANDLE) -> Self {
        Self {
            io: Some((pty_read, pty_write)),
            pty,
            proc,
        }
    }
    pub fn get_io_handles(&mut self) -> Result<(PtyReader, PtyWriter)> {
        self.io.take().chain_err(||"IO handles already taken")
    }
    pub fn set_kill_timeout(&mut self, _timeout_ms: Option<u64>) {
        // unimplemented!()
    }
    pub fn exit_status(&self) -> Option<ExitStatus> {
        unimplemented!()
    }
}