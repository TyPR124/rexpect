#![cfg(windows)]

pub mod process;
pub mod session;




#[cfg(windows)]
#[test]
fn windows_base_test() -> Result<(), String> {
    use winapi::shared::winerror::{
        S_OK
    };
    use winapi::um::{
        consoleapi::{CreatePseudoConsole, ClosePseudoConsole},
        errhandlingapi::{GetLastError},
        handleapi::{CloseHandle},
        namedpipeapi::{CreatePipe},
        wincontypes::COORD,
        winnt::{HANDLE},
    };
    struct ClosableHandle(HANDLE);
    impl Drop for ClosableHandle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0); }
        }
    }
    impl Default for ClosableHandle {
        fn default() -> Self {
            Self(0 as HANDLE)
        }
    }
    impl ClosableHandle {
        pub fn handle(&self) -> &HANDLE {
            &self.0
        }
        pub fn handle_mut(&mut self) -> &mut HANDLE {
            &mut self.0
        }
    }

    // Create pipes
    let mut hPtyInput = ClosableHandle::default();
    let mut hPtyOutput = ClosableHandle::default();
    let mut hProcInput = ClosableHandle::default();
    let mut hProcOutput = ClosableHandle::default();
    let mut hPty = ClosableHandle::default();
    unsafe {
        if CreatePipe(hPtyInput.handle_mut(), hProcOutput.handle_mut(), 0 as *mut _, 0) == 0 {
            Err("cannot create proc->pty pipe")?;
        }
        if CreatePipe(hProcInput.handle_mut(), hPtyOutput.handle_mut(), 0 as *mut _, 0) == 0 {
            Err("cannot create pty->proc pipe")?;
        }
        let r = CreatePseudoConsole(COORD { X: 100, Y: 100 }, *hPtyInput.handle(), *hPtyOutput.handle(), 0, hPty.handle_mut());
        if r != S_OK {
            Err(format!("cannot create pseudoconsole: error {:#X}", r))?
        }
    }

    // proc -> pty
    // pty -> proc ?
    

    Ok(())
}