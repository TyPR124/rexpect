#![cfg(windows)]

pub mod process;
pub mod session;




#[cfg(windows)]
#[test]
fn windows_base_test() -> Result<(), &'static str> {
    use winapi::um::{
        consoleapi::{CreatePseudoConsole, ClosePseudoConsole},
        namedpipeapi::{CreatePipe},
        winnt::{HANDLE},
    };

    // Create pipes
    let hPtyInput = 0 as HANDLE;
    let hPtyOutput = 0 as HANDLE;
    let hProcInput = 0 as HANDLE;
    let hProcOutput = 0 as HANDLE;

    // proc -> pty
    // pty -> proc ?
    

    Ok(())
}