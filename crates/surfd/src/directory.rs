//! The directory `$XDG_RUNTIME_DIR/surf/`, its lock and what is left in it by a `surfd` that
//! ended without removing it.
//!
//! Nothing here follows a symbolic link: every entry is looked at with `symlink_metadata`, a
//! directory is created with `mkdir`, which does not follow a link in its last component, and a
//! file that is opened is checked to be the file that was looked at. A symbolic link anywhere
//! under `surf/` makes `surfd` refuse to start, naming it, and is never removed: in a directory
//! only the user can write, something that is not `surfd` put it there.

use std::fs::{self, DirBuilder, File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use surf_agent::wire;

/// The mode of `surf/` and of every session directory.
pub const MODE: u32 = 0o700;

/// The name of the lock file in `surf/`.
pub const LOCK: &str = "surfd.lock";

/// The name of the file in `surf/` that holds the live view's address and token, one line
/// `http://127.0.0.1:<port>/#<token>`: mode 0600, written at the start, removed at the end.
pub const VIEW_URL: &str = "view-url";

/// The mode of [`VIEW_URL`].
pub const VIEW_URL_MODE: u32 = 0o600;

/// Write `line` to `surf/view-url`, with the lock held: a regular file of that name, left by a
/// `surfd` that was killed, is removed first; anything else of that name (a symbolic link, a
/// directory) refuses, and is not touched. The file is created with `create_new` and mode 0600,
/// and its mode set to 0600 after the creation whatever the umask took off. A reason names the
/// path, never what `line` holds.
pub fn write_view_url(surf: &Path, line: &str) -> Result<PathBuf, String> {
    let path = surf.join(VIEW_URL);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            fs::remove_file(&path).map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
        }
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!("{} is a symbolic link", path.display()));
        }
        Ok(_) => return Err(format!("{} is not a regular file", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(looked(&path, &error)),
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(VIEW_URL_MODE)
        .open(&path)
        .map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let written = file
        .set_permissions(fs::Permissions::from_mode(VIEW_URL_MODE))
        .and_then(|()| file.write_all(line.as_bytes()))
        .and_then(|()| file.flush());
    if let Err(error) = written {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(format!("cannot write {}: {error}", path.display()));
    }
    Ok(path)
}

/// Remove `surf/view-url` if it is a regular file.
pub fn remove_view_url(surf: &Path) -> io::Result<()> {
    let path = surf.join(VIEW_URL);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => fs::remove_file(&path),
        _ => Ok(()),
    }
}

/// The decision on one directory `surfd` uses, `surf/` or a session directory: refused, with the
/// reason, when it is a symbolic link, not a directory, owned by another user than `caller`, or
/// of another mode than 0700. One function over the metadata and the caller's user id
/// ([`wire::caller_uid`]), so that the owner case is decided without a second user.
pub fn decide(path: &Path, metadata: &Metadata, caller: u32) -> Result<(), String> {
    let kind = metadata.file_type();
    if kind.is_symlink() {
        return Err(format!("{} is a symbolic link", path.display()));
    }
    if !kind.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }
    if metadata.uid() != caller {
        return Err(format!(
            "{} is owned by user {}, not by the caller, user {caller}",
            path.display(),
            metadata.uid()
        ));
    }
    let mode = metadata.mode() & 0o7777;
    if mode != MODE {
        return Err(format!(
            "{} has mode {mode:04o}; it must be 0700",
            path.display()
        ));
    }
    Ok(())
}

/// `<runtime>/surf`, created with mode 0700 when absent, and checked by [`decide`].
pub fn prepare(runtime: &Path, caller: u32) -> Result<PathBuf, String> {
    let surf = runtime.join(wire::DIRECTORY);
    match DirBuilder::new().mode(MODE).create(&surf) {
        Ok(()) => {
            // The process's umask may have taken bits off; the directory is ours and new.
            let metadata = fs::symlink_metadata(&surf).map_err(|e| looked(&surf, &e))?;
            if metadata.is_dir() && metadata.mode() & 0o7777 != MODE {
                fs::set_permissions(&surf, fs::Permissions::from_mode(MODE))
                    .map_err(|e| format!("cannot set the mode of {}: {e}", surf.display()))?;
            }
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(format!("cannot create {}: {error}", surf.display())),
    }
    let metadata = fs::symlink_metadata(&surf).map_err(|e| looked(&surf, &e))?;
    decide(&surf, &metadata, caller)?;
    Ok(surf)
}

fn looked(path: &Path, error: &io::Error) -> String {
    format!("cannot look at {}: {error}", path.display())
}

/// The lock file, opened, and whether this process holds its lock.
pub enum Lock {
    /// The exclusive lock is held, for as long as the file is open.
    Held(File),
    /// Another process holds it; the file stays open to try again.
    Busy(File),
}

/// Open or create `surf/surfd.lock` (a regular file of the caller, not a symbolic link) and try
/// its exclusive lock once.
pub fn lock(surf: &Path, caller: u32) -> Result<Lock, String> {
    let path = surf.join(LOCK);
    let file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let before = fs::symlink_metadata(&path).map_err(|e| looked(&path, &e))?;
            if before.file_type().is_symlink() {
                return Err(format!("{} is a symbolic link", path.display()));
            }
            if !before.is_file() {
                return Err(format!("{} is not a regular file", path.display()));
            }
            if before.uid() != caller {
                return Err(format!(
                    "{} is owned by user {}, not by the caller, user {caller}",
                    path.display(),
                    before.uid()
                ));
            }
            let file = OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
            let opened = file.metadata().map_err(|e| looked(&path, &e))?;
            if opened.dev() != before.dev() || opened.ino() != before.ino() {
                return Err(format!("{} changed while it was opened", path.display()));
            }
            file
        }
        Err(error) => return Err(format!("cannot create {}: {error}", path.display())),
    };
    try_lock(file, &path)
}

/// Try the exclusive lock of `file` once more.
pub fn try_lock(file: File, path: &Path) -> Result<Lock, String> {
    match file.try_lock() {
        Ok(()) => Ok(Lock::Held(file)),
        Err(fs::TryLockError::WouldBlock) => Ok(Lock::Busy(file)),
        Err(fs::TryLockError::Error(error)) => {
            Err(format!("cannot lock {}: {error}", path.display()))
        }
    }
}

/// Remove what a `surfd` that ended left in `surf/`, with the lock held: a `surfd.sock` that is a
/// socket refusing a connection, and every session directory (named by a lower-case UUID). Every
/// entry is checked before anything is removed. Refused, and nothing removed, when an entry is a
/// symbolic link, when `surfd.sock` is not a socket or answers, or when a session directory
/// fails [`decide`] or holds a symbolic link. Other entries are left alone.
pub fn remove_stale(surf: &Path, caller: u32) -> Result<(), String> {
    let entries = fs::read_dir(surf).map_err(|e| format!("cannot read {}: {e}", surf.display()))?;
    let mut stale_socket = None;
    let mut sessions = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", surf.display()))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|e| looked(&path, &e))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{} is a symbolic link", path.display()));
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name == LOCK {
            continue;
        }
        if name == wire::SOCKET {
            if !metadata.file_type().is_socket() {
                return Err(format!("{} is not a socket", path.display()));
            }
            match UnixStream::connect(&path) {
                Ok(_) => {
                    return Err(format!(
                        "{} answers, but no surfd holds the lock",
                        path.display()
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
                    stale_socket = Some(path);
                }
                Err(error) => return Err(format!("cannot check {}: {error}", path.display())),
            }
            continue;
        }
        if wire::session_id(name).as_deref() == Some(name) && metadata.is_dir() {
            decide(&path, &metadata, caller)?;
            check_tree(&path)?;
            sessions.push(path);
        }
    }
    if let Some(socket) = stale_socket {
        fs::remove_file(&socket).map_err(|e| format!("cannot remove {}: {e}", socket.display()))?;
    }
    for session in sessions {
        remove_tree(&session)?;
    }
    Ok(())
}

/// Refused, naming it, when anything under `dir` is a symbolic link.
pub fn check_tree(dir: &Path) -> Result<(), String> {
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current)
            .map_err(|e| format!("cannot read {}: {e}", current.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("cannot read {}: {e}", current.display()))?;
            let kind = entry.file_type().map_err(|e| looked(&entry.path(), &e))?;
            if kind.is_symlink() {
                return Err(format!("{} is a symbolic link", entry.path().display()));
            }
            if kind.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(())
}

/// Remove `dir` and everything under it, following no symbolic link and removing none: refused,
/// with nothing removed, when [`check_tree`] finds one.
pub fn remove_tree(dir: &Path) -> Result<(), String> {
    check_tree(dir)?;
    remove_checked(dir).map_err(|e| format!("cannot remove {}: {e}", dir.display()))
}

fn remove_checked(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::other("a symbolic link appeared"));
        }
        if kind.is_dir() {
            remove_checked(&entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    fs::remove_dir(dir)
}

/// Create the session directory `dir` with mode 0700. Refused when anything is at that path
/// already, a symbolic link included (`mkdir` does not follow it), and when the directory made is
/// not one [`decide`] accepts.
pub fn create_session_dir(dir: &Path, caller: u32) -> Result<(), String> {
    match DirBuilder::new().mode(MODE).create(dir) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(format!("{} exists before surfd created it", dir.display()));
        }
        Err(error) => return Err(format!("cannot create {}: {error}", dir.display())),
    }
    let metadata = fs::symlink_metadata(dir).map_err(|e| looked(dir, &e))?;
    if metadata.is_dir() && metadata.mode() & 0o7777 != MODE {
        fs::set_permissions(dir, fs::Permissions::from_mode(MODE))
            .map_err(|e| format!("cannot set the mode of {}: {e}", dir.display()))?;
    }
    let metadata = fs::symlink_metadata(dir).map_err(|e| looked(dir, &e))?;
    if let Err(reason) = decide(dir, &metadata, caller) {
        let _ = fs::remove_dir(dir);
        return Err(reason);
    }
    Ok(())
}
