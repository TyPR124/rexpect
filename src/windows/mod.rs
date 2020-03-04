#![cfg(windows)]

mod pipe;
mod command;
mod process;
pub use command::Command;
pub use process::PtyProcess;

pub type PtyReader = pipe::Receiver;
pub type PtyWriter = pipe::Sender;
