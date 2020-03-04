#![cfg(unix)]

mod process;

pub type PtyReader = std::fs::File;
pub type PtyWriter = std::fs::File;
pub type Command = std::process::Command;
pub use process::{ProcessExt, PtyProcess};