//! Start a process via pty

// use std;
use std::fs::File;
use std::process::ExitStatus;
// use std::os::unix::process::ExitStatusExt;
// use std::os::unix::io::{FromRawFd, AsRawFd};

// use nix::pty::{posix_openpt, grantpt, unlockpt, PtyMaster};
// use nix::fcntl::{OFlag, open};
// use nix;
// use nix::sys::{stat, termios};
// use nix::unistd::{fork, ForkResult, setsid, dup, dup2, Pid};
// use nix::libc::{STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO};

#[cfg(unix)] // TODO: Move this somewhere else
pub use nix::sys::{wait, signal};

use crate::errors::*; // load error-chain
use crate::Command;

#[cfg(unix)]
use crate::unix as imp;
#[cfg(windows)]
use crate::windows as imp;

use imp::{PtyReader, PtyWriter};

/// Start a process in a forked tty so you can interact with it the same as you would
/// within a terminal
///
/// The process and pty session are killed upon dropping PtyProcess
///
/// # Example
///
/// Typically you want to do something like this (for a more complete example see
/// unit test `test_cat` within this module):
///
/// ```no_run
/// # #![allow(unused_mut)]
/// # #![allow(unused_variables)]
///
/// //extern crate nix;
/// //extern crate rexpect;
///
/// use rexpect::process::PtyProcess;
/// use rexpect::os::unix::ProcessExt;
/// use rexpect::Command;
/// //use std::process::Command;
/// use std::fs::File;
/// use std::io::{BufReader, LineWriter};
/// use std::os::unix::io::{FromRawFd, AsRawFd};
/// use nix::unistd::dup;
///
/// # fn main() {
///
/// let mut process = PtyProcess::new(Command::new("cat")).expect("could not execute cat");
/// //let fd = dup(process.inner.pty.as_raw_fd()).unwrap();
/// //let f = unsafe { File::from_raw_fd(fd) };
/// let f = process.get_file_handle();
/// let mut writer = LineWriter::new(&f);
/// let mut reader = BufReader::new(&f);
/// process.exit().expect("could not terminate process");
///
/// // writer.write() sends strings to `cat`
/// // writer.reader() reads back what `cat` wrote
/// // send Ctrl-C with writer.write(&[3]) and writer.flush()
///
/// # }
/// ```
pub struct PtyProcess {
    // pub(crate) is for testing
    pub(crate) inner: imp::PtyProcess
}


// #[cfg(target_os = "linux")]
// use nix::pty::ptsname_r;

// #[cfg(target_os = "macos")]
// /// ptsname_r is a linux extension but ptsname isn't thread-safe
// /// instead of using a static mutex this calls ioctl with TIOCPTYGNAME directly
// /// based on https://blog.tarq.io/ptsname-on-osx-with-rust/
// fn ptsname_r(fd: &PtyMaster) -> nix::Result<String> {
//     use std::ffi::CStr;
//     use std::os::unix::io::AsRawFd;
//     use nix::libc::{ioctl, TIOCPTYGNAME};

//     /// the buffer size on OSX is 128, defined by sys/ttycom.h
//     let buf: [i8; 128] = [0; 128];

//     unsafe {
//         match ioctl(fd.as_raw_fd(), TIOCPTYGNAME as u64, &buf) {
//             0 => {
//                 let res = CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned();
//                 Ok(res)
//             }
//             _ => Err(nix::Error::last()),
//         }
//     }
// }

impl PtyProcess {
    /// Start a process in a forked pty
    pub fn new(command: Command) -> Result<Self> {
        let inner = imp::PtyProcess::new(command.into_inner())?;
        Ok(Self { inner })
    }

    // /// Get handle to pty fork for reading/writing
    // pub fn get_file_handle(&self) -> File {
    //     // self.inner.get_file_handle()
    //     unimplemented!("Move get_file_handle to unix ext trait")
    // }

    pub(crate) fn get_io_handles(&mut self) -> Result<(PtyReader, PtyWriter)> {
        self.inner.get_io_handles()
    }

    /// At the drop of PtyProcess the running process is killed. This is blocking forever if
    /// the process does not react to a normal kill. If kill_timeout is set the process is
    /// `kill -9`ed after duration
    pub fn set_kill_timeout(&mut self, timeout_ms: Option<u64>) {
        self.inner.set_kill_timeout(timeout_ms)
    }

    // Now in os::unix::ProcessExt
    // /// Get status of child process, nonblocking.
    // ///
    // /// This method runs waitpid on the process.
    // /// This means: If you ran `exit()` before or `status()` tihs method will
    // /// return an Error
    // ///
    // /// # Example
    // /// ```rust,no_run
    // ///
    // /// # extern crate rexpect;
    // /// use rexpect::process;
    // /// use std::process::Command;
    // ///
    // /// # fn main() {
    // ///     let cmd = Command::new("/path/to/myprog");
    // ///     let process = process::PtyProcess::new(cmd).expect("could not execute myprog");
    // ///     while process.status().unwrap() == process::wait::WaitStatus::StillAlive {
    // ///         // do something
    // ///     }
    // /// # }
    // /// ```
    // ///
    // pub fn status(&self) -> Option<wait::WaitStatus> {
    //     self.inner.status()
    // }

    pub fn exit_status(&self) -> Option<ExitStatus> {
        self.inner.exit_status()
    }
    // Now in os::unix::ProcessExt
    // /// Wait until process has exited. This is a blocking call.
    // /// If the process doesn't terminate this will block forever.
    // pub fn wait(&self) -> Result<wait::WaitStatus> {
    //     self.inner.wait()
    // }

    // Now in os::unix::ProcessExt
    // /// Regularly exit the process, this method is blocking until the process is dead
    // pub fn exit(&mut self) -> Result<wait::WaitStatus> {
    //     self.inner.exit()
    // }

    // Now in os::unix::ProcessExt
    // /// Nonblocking variant of `kill()` (doesn't wait for process to be killed)
    // pub fn signal(&mut self, sig: signal::Signal) -> Result<()> {
    //     self.inner.signal(sig)
    // }

    // Now in os::unix::ProcessExt
    // /// Kill the process with a specific signal. This method blocks, until the process is dead
    // ///
    // /// repeatedly sends SIGTERM to the process until it died,
    // /// the pty session is closed upon dropping PtyMaster,
    // /// so we don't need to explicitely do that here.
    // ///
    // /// if `kill_timeout` is set and a repeated sending of signal does not result in the process
    // /// being killed, then `kill -9` is sent after the `kill_timeout` duration has elapsed.
    // pub fn kill(&mut self, sig: signal::Signal) -> Result<wait::WaitStatus> {
    //     self.inner.kill(sig)
    // }
}

#[cfg(all(unix, test))]
mod tests {
    use super::*;
    use std::io::{BufReader, LineWriter};
    use nix::sys::{wait, signal};
    use std::io::prelude::*;
    use std::{thread, time};
    // use crate::os::unix::ProcessExt;

    #[test]
    /// Open cat, write string, read back string twice, send Ctrl^C and check that cat exited
    fn test_cat() {
        // wrapping into closure so I can use ?
        || -> std::io::Result<()> {
            use crate::os::unix::ProcessExt;
            let process = PtyProcess::new(Command::new("cat")).expect("could not execute cat");
            let f = process.get_file_handle();
            let mut writer = LineWriter::new(&f);
            let mut reader = BufReader::new(&f);
            writer.write(b"hello cat\n")?;
            let mut buf = String::new();
            reader.read_line(&mut buf)?;
            assert_eq!(buf, "hello cat\r\n");

            // this sleep solves an edge case of some cases when cat is somehow not "ready"
            // to take the ^C (occasional test hangs)
            thread::sleep(time::Duration::from_millis(100));
            writer.write_all(&[3])?; // send ^C
            writer.flush()?;
            let should =
                wait::WaitStatus::Signaled(process.inner.child_pid, signal::Signal::SIGINT, false);
            assert_eq!(should, wait::waitpid(process.inner.child_pid, None).unwrap());
            Ok(())
        }()
                .unwrap_or_else(|e| panic!("test_cat failed: {}", e));
    }
}
