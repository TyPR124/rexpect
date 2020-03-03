#![cfg(unix)]

pub mod process;

pub type PtyReader = std::fs::File;
pub type PtyWriter = std::fs::File;