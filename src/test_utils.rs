use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

pub static ENV_MUTEX: Mutex<()> = Mutex::new(());

pub fn lock_env() -> MutexGuard<'static, ()> {
    ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

#[must_use]
pub struct TestProcess {
    _lock: MutexGuard<'static, ()>,
    original_cwd: PathBuf,
    original_vars: HashMap<OsString, Option<OsString>>,
}

impl TestProcess {
    pub fn new() -> Self {
        let lock = lock_env();
        let original_cwd = env::current_dir().expect("current dir");
        Self {
            _lock: lock,
            original_cwd,
            original_vars: HashMap::new(),
        }
    }

    pub fn chdir(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        env::set_current_dir(path)
    }

    pub fn set_var(&mut self, key: impl Into<OsString>, value: impl AsRef<OsStr>) {
        let key = key.into();
        self.remember_var(&key);
        unsafe {
            env::set_var(&key, value);
        }
    }

    pub fn remove_var(&mut self, key: impl Into<OsString>) {
        let key = key.into();
        self.remember_var(&key);
        unsafe {
            env::remove_var(&key);
        }
    }

    fn remember_var(&mut self, key: &OsStr) {
        if self.original_vars.contains_key(key) {
            return;
        }
        self.original_vars
            .insert(key.to_os_string(), env::var_os(key));
    }
}

impl Drop for TestProcess {
    fn drop(&mut self) {
        for (key, previous) in self.original_vars.drain() {
            if let Some(value) = previous {
                unsafe {
                    env::set_var(&key, value);
                }
            } else {
                unsafe {
                    env::remove_var(&key);
                }
            }
        }
        let _ = env::set_current_dir(&self.original_cwd);
    }
}
