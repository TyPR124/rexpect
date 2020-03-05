use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::{self, ErrorKind};
use std::mem::{align_of, size_of, size_of_val};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

use lazy_static::lazy_static;
use static_assertions::const_assert;

use winapi::{
    shared::winerror::{S_OK, HRESULT_CODE},
    um::{
        consoleapi::{ClosePseudoConsole, CreatePseudoConsole},
        handleapi::CloseHandle,
        processthreadsapi::{
            CreateProcessW, InitializeProcThreadAttributeList, UpdateProcThreadAttribute,
            PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_LIST,
        },
        winbase::{CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT, STARTUPINFOEXW},
        wincontypes::COORD,
        winnt::{HANDLE, VOID},
    },
};

use crate::errors::Result;
use super::{PtyProcess, pipe};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[doc(hidden)]
pub struct EnvKey(OsString);

impl From<OsString> for EnvKey {
    fn from(_k: OsString) -> Self {
        // let mut buf = k.into_inner().into_inner();
        // buf.make_ascii_uppercase();
        // EnvKey(FromInner::from_inner(FromInner::from_inner(buf)))
        unimplemented!()
    }
}

impl From<EnvKey> for OsString {
    fn from(k: EnvKey) -> Self {
        k.0
    }
}

impl Borrow<OsStr> for EnvKey {
    fn borrow(&self) -> &OsStr {
        &self.0
    }
}

impl AsRef<OsStr> for EnvKey {
    fn as_ref(&self) -> &OsStr {
        &self.0
    }
}

// Stores a set of changes to an environment
#[derive(Clone, Debug)]
pub struct CommandEnv {
    clear: bool,
    saw_path: bool,
    vars: BTreeMap<EnvKey, Option<OsString>>,
}

impl Default for CommandEnv {
    fn default() -> Self {
        CommandEnv {
            clear: false,
            saw_path: false,
            vars: Default::default(),
        }
    }
}

impl CommandEnv {
    // Capture the current environment with these changes applied
    pub fn capture(&self) -> BTreeMap<EnvKey, OsString> {
        let mut result = BTreeMap::<EnvKey, OsString>::new();
        if !self.clear {
            for (k, v) in env::vars_os() {
                result.insert(k.into(), v);
            }
        }
        for (k, maybe_v) in &self.vars {
            if let &Some(ref v) = maybe_v {
                result.insert(k.clone(), v.clone());
            } else {
                result.remove(k);
            }
        }
        result
    }

    // Apply these changes directly to the current environment
    // pub fn apply(&self) {
    //     if self.clear {
    //         for (k, _) in env::vars_os() {
    //             env::remove_var(k);
    //         }
    //     }
    //     for (key, maybe_val) in self.vars.iter() {
    //         if let &Some(ref val) = maybe_val {
    //             env::set_var(key, val);
    //         } else {
    //             env::remove_var(key);
    //         }
    //     }
    // }

    pub fn is_unchanged(&self) -> bool {
        !self.clear && self.vars.is_empty()
    }

    pub fn capture_if_changed(&self) -> Option<BTreeMap<EnvKey, OsString>> {
        if self.is_unchanged() {
            None
        } else {
            Some(self.capture())
        }
    }

    // The following functions build up changes
    pub fn set(&mut self, key: &OsStr, value: &OsStr) {
        self.maybe_saw_path(&key);
        self.vars
            .insert(key.to_owned().into(), Some(value.to_owned()));
    }

    pub fn remove(&mut self, key: &OsStr) {
        self.maybe_saw_path(&key);
        if self.clear {
            self.vars.remove(key);
        } else {
            self.vars.insert(key.to_owned().into(), None);
        }
    }

    pub fn clear(&mut self) {
        self.clear = true;
        self.vars.clear();
    }

    // pub fn have_changed_path(&self) -> bool {
    //     self.saw_path || self.clear
    // }

    fn maybe_saw_path(&mut self, key: &OsStr) {
        if !self.saw_path && key == "PATH" {
            self.saw_path = true;
        }
    }
}


fn ensure_no_nuls<T: AsRef<OsStr>>(str: T) -> io::Result<T> {
    if str.as_ref().encode_wide().any(|b| b == 0) {
        Err(io::Error::new(
            ErrorKind::InvalidInput,
            "nul byte found in provided data",
        ))
    } else {
        Ok(str)
    }
}

pub struct Command {
    program: OsString,
    args: Vec<OsString>,
    env: CommandEnv,
    cwd: Option<OsString>,
    // flags: u32,
    // detach: bool, // not currently exposed in std::process
    // Don't need to track stdin/out/err, using Pty for that
    // stdin: Option<Stdio>,
    // stdout: Option<Stdio>,
    // stderr: Option<Stdio>,
}


impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        Command {
            program: program.as_ref().to_os_string(),
            args: Vec::new(),
            env: Default::default(),
            cwd: None,
            // flags: 0,
            // detach: false,
            // stdin: None,
            // stdout: None,
            // stderr: None,
        }
    }

    pub fn spawn_pty(&self) -> io::Result<PtyProcess> {
        let maybe_env = self.env.capture_if_changed();
        // To have the spawning semantics of unix/windows stay the same, we need
        // to read the *child's* PATH if one is provided. See #15149 for more
        // details.
        let program = maybe_env.as_ref().and_then(|env| {
            if let Some(v) = env.get(OsStr::new("PATH")) {
                // Split the value and test each path to see if the
                // program exists.
                for path in env::split_paths(&v) {
                    let path = path
                        .join(self.program.to_str().unwrap())
                        .with_extension(env::consts::EXE_EXTENSION);
                    if fs::metadata(&path).is_ok() {
                        return Some(path.into_os_string());
                    }
                }
            }
            None
        });

        // let mut si = zeroed_startupinfo();
        // si.cb = mem::size_of::<c::STARTUPINFO>() as c::DWORD;
        // si.dwFlags = c::STARTF_USESTDHANDLES;

        let mut si = STARTUPINFOEXW::default();
        si.StartupInfo.cb = size_of_val(&si) as u32;
        
        let (input_tx, input_rx) = pipe::unnamed()?;
        let (output_tx, output_rx) = pipe::unnamed()?;
        let size = COORD { X: 120, Y: 120 };
        let mut pty_handle = ptr::null_mut();
        let r = unsafe {
            CreatePseudoConsole(
                size,
                input_rx.as_raw_handle(),
                output_tx.as_raw_handle(),
                0,
                &mut pty_handle,
            )
        };
        if r != S_OK {
            Err(io::Error::from_raw_os_error(HRESULT_CODE(r)))?;
        }
        let mut boxed_tal = make_boxed_tal()?;
        fill_tal(&mut boxed_tal, pty_handle)?;
        si.lpAttributeList = boxed_tal.as_mut_ptr().cast();

        let program = program.as_ref().unwrap_or(&self.program);
        let mut cmd_str = make_command_line(program, &self.args)?;
        cmd_str.push(0); // add null terminator

        // stolen from the libuv code.
        // let mut flags = self.flags | c::CREATE_UNICODE_ENVIRONMENT;
        // if self.detach {
        //     flags |= c::DETACHED_PROCESS | c::CREATE_NEW_PROCESS_GROUP;
        // }
        let flags = CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT;

        let (envp, _data) = make_envp(maybe_env)?;
        let (dirp, _data) = make_dirp(self.cwd.as_ref())?;
        let mut pi = PROCESS_INFORMATION::default();

        lazy_static! {
            static ref CREATE_PROCESS_LOCK: Mutex<()> = Mutex::new(());
        }
        let _guard = CREATE_PROCESS_LOCK
            .lock()
            .expect("CREATE_PROCESS_LOCK error");

        let r = unsafe {
            CreateProcessW(
                ptr::null_mut(), // app name (if null, taken from cmd_str)
                cmd_str.as_mut_ptr(),
                ptr::null_mut(), // proc attr
                ptr::null_mut(), // thread attr
                0,               // Inherit handles bool,
                flags,
                envp,
                dirp,
                &mut si.StartupInfo,
                &mut pi,
            )
        };

        if r == 0 {
            Err(io::Error::last_os_error())?;
        }

        drop(_guard);
        unsafe { CloseHandle(pi.hThread) };
        let proc_handle = pi.hProcess;

        Ok(PtyProcess::init(output_rx, input_tx, pty_handle, proc_handle))
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.as_ref().to_os_string());
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }
    fn env_mut(&mut self) -> &mut CommandEnv {
        &mut self.env
    }
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env_mut().set(key.as_ref(), val.as_ref());
        self
    }
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (ref key, ref val) in vars {
            self.env_mut().set(key.as_ref(), val.as_ref());
        }
        self
    }
    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
        self.env_mut().remove(key.as_ref());
        self
    }
    pub fn env_clear(&mut self) -> &mut Command {
        self.env_mut().clear();
        self
    }
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.cwd(dir.as_ref().as_ref());
        self
    }
    fn cwd(&mut self, dir: &OsStr) {
        self.cwd = Some(dir.to_os_string())
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.program)?;
        for arg in &self.args {
            write!(f, " {:?}", arg)?;
        }
        Ok(())
    }
}

// Produces a wide string *without terminating null*; returns an error if
// `prog` or any of the `args` contain a nul.
fn make_command_line(prog: &OsStr, args: &[OsString]) -> io::Result<Vec<u16>> {
    // Encode the command and arguments in a command line string such
    // that the spawned process may recover them using CommandLineToArgvW.
    let mut cmd: Vec<u16> = Vec::new();
    // Always quote the program name so CreateProcess doesn't interpret args as
    // part of the name if the binary wasn't found first time.
    append_arg(&mut cmd, prog, true)?;
    for arg in args {
        cmd.push(' ' as u16);
        append_arg(&mut cmd, arg, false)?;
    }
    return Ok(cmd);

    fn append_arg(cmd: &mut Vec<u16>, arg: &OsStr, force_quotes: bool) -> io::Result<()> {
        // If an argument has 0 characters then we need to quote it to ensure
        // that it actually gets passed through on the command line or otherwise
        // it will be dropped entirely when parsed on the other end.
        ensure_no_nuls(arg)?;
        // let arg_bytes = &arg.as_inner().inner.as_inner();
        // let quote = force_quotes
        //     || arg_bytes.iter().any(|c| *c == b' ' || *c == b'\t')
        //     || arg_bytes.is_empty();
        let mut arg_bytes = arg.encode_wide();
        let quote = force_quotes
            || arg_bytes
                .clone()
                .any(|c| c == ' ' as u16 || c == '\t' as u16)
            || arg_bytes.next().is_none();
        if quote {
            cmd.push('"' as u16);
        }

        let mut backslashes: usize = 0;
        for x in arg.encode_wide() {
            if x == '\\' as u16 {
                backslashes += 1;
            } else {
                if x == '"' as u16 {
                    // Add n+1 backslashes to total 2n+1 before internal '"'.
                    cmd.extend((0..=backslashes).map(|_| '\\' as u16));
                }
                backslashes = 0;
            }
            cmd.push(x);
        }

        if quote {
            // Add n backslashes to total 2n before ending '"'.
            cmd.extend((0..backslashes).map(|_| '\\' as u16));
            cmd.push('"' as u16);
        }
        Ok(())
    }
}

fn make_envp(maybe_env: Option<BTreeMap<EnvKey, OsString>>) -> io::Result<(*mut VOID, Vec<u16>)> {
    // On Windows we pass an "environment block" which is not a char**, but
    // rather a concatenation of null-terminated k=v\0 sequences, with a final
    // \0 to terminate.
    if let Some(env) = maybe_env {
        let mut blk = Vec::new();

        for (k, v) in env {
            blk.extend(ensure_no_nuls(k.0)?.encode_wide());
            blk.push('=' as u16);
            blk.extend(ensure_no_nuls(v)?.encode_wide());
            blk.push(0);
        }
        blk.push(0);
        Ok((blk.as_mut_ptr() as *mut VOID, blk))
    } else {
        Ok((ptr::null_mut(), Vec::new()))
    }
}

fn make_dirp(d: Option<&OsString>) -> io::Result<(*const u16, Vec<u16>)> {
    match d {
        Some(dir) => {
            let mut dir_str: Vec<u16> = ensure_no_nuls(dir)?.encode_wide().collect();
            dir_str.push(0);
            Ok((dir_str.as_ptr(), dir_str))
        }
        None => Ok((ptr::null(), Vec::new())),
    }
}

#[allow(non_camel_case_types)]
type TAL_BUF_UNIT = u64;
const_assert!(align_of::<TAL_BUF_UNIT>() >= align_of::<PROC_THREAD_ATTRIBUTE_LIST>());
const TAL_BUF_UNIT_SIZE: usize = size_of::<TAL_BUF_UNIT>();

fn make_boxed_tal() -> io::Result<Box<[TAL_BUF_UNIT]>> {
    let mut tal_size_bytes = 0;
    // No need to check return value, call will fail but fill in tal_size value.
    unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut tal_size_bytes) };
    let tal_size_bytes = tal_size_bytes as usize;
    let tal_size_units = match tal_size_bytes % TAL_BUF_UNIT_SIZE {
        0 => tal_size_bytes / TAL_BUF_UNIT_SIZE,
        _ => (tal_size_bytes / TAL_BUF_UNIT_SIZE) + 1,
    };
    let mut tal_buf = Vec::<TAL_BUF_UNIT>::with_capacity(tal_size_units);
    tal_buf.resize(tal_size_units, 0);
    let mut tal_buf = tal_buf.into_boxed_slice();
    //Actually init in the TAL
    let r = unsafe {
        InitializeProcThreadAttributeList(
            tal_buf.as_mut_ptr().cast(),
            1,
            0,
            &mut tal_size_bytes.clone(),
        )
    };
    if r == 0 {
        Err(io::Error::last_os_error())?;
    }
    Ok(tal_buf)
}

fn fill_tal(tal_buf: &mut [TAL_BUF_UNIT], pty: HANDLE) -> io::Result<()> {
    // Magic value comes from WinBase.h, value is currently not implemented in winapi-rs
    const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 0x0002_0016;
    let r = unsafe {
        UpdateProcThreadAttribute(
            tal_buf.as_mut_ptr().cast(),
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            pty,
            size_of_val(&pty),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if r == 0 {
        Err(io::Error::last_os_error())?;
    }
    Ok(())
}
