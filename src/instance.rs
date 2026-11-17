use anyhow::{Context, Result, bail};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

/// SingleInstanceLock guarantees that only one instance of PhoneCam
/// runs at a time using Linux kernel file locking (`flock`).
pub struct SingleInstanceLock {
    file: File,
    path: PathBuf,
}

impl SingleInstanceLock {
    /// Attempt to acquire the single-instance lock.
    /// Returns an error if another instance is already running.
    pub fn acquire() -> Result<Self> {
        let lock_path = Self::default_lock_path();
        Self::acquire_at(lock_path)
    }

    /// Attempt to acquire the lock at a specific path (useful for testing)
    pub fn acquire_at(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&path)
            .with_context(|| format!("Failed to open lockfile at {:?}", path))?;

        let fd = file.as_raw_fd();
        let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EWOULDBLOCK)
                || err.raw_os_error() == Some(libc::EAGAIN)
            {
                // Read PID of existing instance from lockfile
                let mut content = String::new();
                if let Ok(mut read_file) = File::open(&path) {
                    let _ = read_file.read_to_string(&mut content);
                }
                let pid_str = content.trim();
                if !pid_str.is_empty() {
                    bail!(
                        "PhoneCam is already running (PID: {}). Only one instance is allowed.",
                        pid_str
                    );
                } else {
                    bail!("PhoneCam is already running. Only one instance is allowed.");
                }
            } else {
                bail!("Failed to acquire single-instance lock: {}", err);
            }
        }

        // Successfully acquired lock. Write our PID.
        let mut file_write = file;
        let _ = file_write.set_len(0);
        let pid = std::process::id();
        let _ = writeln!(file_write, "{}", pid);
        let _ = file_write.flush();

        Ok(SingleInstanceLock {
            file: file_write,
            path,
        })
    }

    /// Determine the default lockfile path using XDG_RUNTIME_DIR or fallback to /tmp
    pub fn default_lock_path() -> PathBuf {
        match std::env::var_os("XDG_RUNTIME_DIR") {
            Some(dir) => PathBuf::from(dir).join("phonecam.lock"),
            None => {
                let uid = unsafe { libc::getuid() };
                PathBuf::from(format!("/tmp/phonecam-{}.lock", uid))
            }
        }
    }

    /// Path of the acquired lock file
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        let fd = self.file.as_raw_fd();
        unsafe {
            libc::flock(fd, libc::LOCK_UN);
        }
        let _ = std::fs::remove_file(&self.path);
    }
}
