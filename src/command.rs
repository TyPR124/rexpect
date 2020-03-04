use std::ffi::OsStr;
use std::path::Path;
use std::fmt::{self, Debug, Formatter};

#[cfg(unix)]
use crate::unix as imp;
#[cfg(windows)]
use crate::windows as imp;

pub struct Command {
    inner: imp::Command
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        Self { inner: imp::Command::new(program) }
    }
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.inner.arg(arg);
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.env(key, val);
        self
    }
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.envs(vars);
        self
    }
    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
        self.inner.env_remove(key);
        self
    }
    pub fn env_clear(&mut self) -> &mut Command {
        self.inner.env_clear();
        self
    }
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.inner.current_dir(dir);
        self
    }
    pub(crate) fn into_inner(self) -> imp::Command { self.inner }
}
impl Debug for Command {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}