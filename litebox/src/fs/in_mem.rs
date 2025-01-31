//! An in-memory file system, not backed by any physical device.

use alloc::string::String;
use hashbrown::HashMap;

use crate::path::Arg;
use crate::sync;

use super::errors::{
    ChmodError, CloseError, MkdirError, OpenError, PathError, ReadError, RmdirError, UnlinkError,
    WriteError,
};
use super::Mode;

/// A backing implementation for [`FileSystem`](super::FileSystem) storing all files in-memory.
///
/// # Warning
///
/// This has no physical backing store, thus any files in memory are erased as soon as this object
/// is dropped.
pub struct FileSystem<'platform, Platform: sync::RawSyncPrimitivesProvider> {
    // TODO: Possibly support a single-threaded variant that doesn't have the cost of requiring a
    // sync-primitives platform, as well as cost of mutexes and such?
    sync: sync::Synchronization<'platform, Platform>,
    root: sync::RwLock<'platform, Platform, RootDir>,
    current_user: UserInfo,
    // cwd invariant: always ends with a `/`
    current_working_dir: String,
}

impl<'platform, Platform: sync::RawSyncPrimitivesProvider> FileSystem<'platform, Platform> {
    /// Construct a new `FileSystem` instance
    ///
    /// This function is expected to only be invoked once per platform, as an initialiation step,
    /// and the created `FileSystem` handle is expected to be shared across all usage over the
    /// system.
    #[must_use]
    pub fn new(platform: &'platform Platform) -> Self {
        let sync = sync::Synchronization::new(platform);
        let root = sync.new_rwlock(RootDir::new());
        Self {
            sync,
            root,
            current_user: UserInfo {
                user: 1000,
                group: 1000,
            },
            current_working_dir: "/".into(),
        }
    }
}

impl<Platform: sync::RawSyncPrimitivesProvider> super::private::Sealed
    for FileSystem<'_, Platform>
{
}

impl<Platform: sync::RawSyncPrimitivesProvider> FileSystem<'_, Platform> {
    // Gives the absolute path for `path`, resolving any `.` or `..`s, and making sure to account
    // for any relative paths from current working directory.
    //
    // Note: does NOT account for symlinks.
    fn absolute_path(&self, path: impl crate::path::Arg) -> Result<String, PathError> {
        // Since cwd always ends with `/`, if the provided path is a relative path, it'll do the
        // right thing; if it is an absolute path, it'll lead to a `//`, which the normalizer will
        // correctly ignore everything before it. Thus, it is sufficient to simply concatenate and
        // normalize.
        assert!(self.current_working_dir.ends_with('/'));
        Ok((self.current_working_dir.clone() + path.as_rust_str()?).normalized()?)
    }
}

impl<Platform: sync::RawSyncPrimitivesProvider> super::FileSystem for FileSystem<'_, Platform> {
    fn open(
        &self,
        path: impl crate::path::Arg,
        flags: super::OFlags,
        mode: super::Mode,
    ) -> Result<crate::fd::FileFd, OpenError> {
        todo!()
    }

    fn close(&self, fd: crate::fd::FileFd) -> Result<(), CloseError> {
        todo!()
    }

    fn read(&self, fd: &crate::fd::FileFd, buf: &mut [u8]) -> Result<usize, ReadError> {
        todo!()
    }

    fn write(&self, fd: &crate::fd::FileFd, buf: &[u8]) -> Result<usize, WriteError> {
        todo!()
    }

    fn chmod(&self, path: impl crate::path::Arg, mode: super::Mode) -> Result<(), ChmodError> {
        todo!()
    }

    fn unlink(&self, path: impl crate::path::Arg) -> Result<(), UnlinkError> {
        todo!()
    }

    fn mkdir(&self, path: impl crate::path::Arg, mode: super::Mode) -> Result<(), MkdirError> {
        let path = self.absolute_path(path)?;
        let mut root = self.root.write();
        let (parent, entry) = root.parent_and_entry_mut(&path, self.current_user)?;
        let Some((parent_path, parent)) = parent else {
            // Attempted to make `/`
            return Err(MkdirError::AlreadyExists);
        };
        let None = entry else {
            return Err(MkdirError::AlreadyExists);
        };
        if !self.current_user.can_write(&parent.perms) {
            return Err(MkdirError::NoWritePerms);
        };
        parent.children_count = parent.children_count.checked_add(1).unwrap();
        let old = root.entries.insert(
            path,
            Entry::Dir(Dir {
                perms: Permissions {
                    mode,
                    userinfo: self.current_user,
                },
                children_count: 0,
            }),
        );
        Ok(())
    }

    fn rmdir(&self, path: impl crate::path::Arg) -> Result<(), RmdirError> {
        let path = self.absolute_path(path)?;
        let mut root = self.root.write();
        let (parent, entry) = root.parent_and_entry_mut(&path, self.current_user)?;
        let Some((_, parent)) = parent else {
            // Attempted to remove `/`
            return Err(RmdirError::Busy);
        };
        let Some(entry) = entry else {
            return Err(PathError::NoSuchFileOrDirectory)?;
        };
        let Entry::Dir(dir) = entry else {
            return Err(RmdirError::NotADirectory);
        };
        if dir.children_count > 0 {
            return Err(RmdirError::NotEmpty);
        }
        if !self.current_user.can_write(&parent.perms) {
            return Err(RmdirError::NoWritePerms);
        }
        parent.children_count = parent.children_count.checked_sub(1).unwrap();
        let removed = root.entries.remove(&path).unwrap();
        // Just a sanity check
        assert!(matches!(
            removed,
            Entry::Dir(Dir {
                children_count: 0,
                ..
            })
        ));
        Ok(())
    }
}

struct RootDir {
    // keys are normalized paths; directories do not have the final `/` (thus the root would be at
    // the empty-string key "")
    entries: HashMap<String, Entry>,
}

// Parent, if it exists, is the path as well as the directory
//
// The entry, if it exists, is just the entry itself
type ParentAndEntry<'a, D, E> = Result<(Option<(&'a str, D)>, Option<E>), PathError>;

impl RootDir {
    fn new() -> Self {
        Self {
            entries: [(
                String::new(),
                Entry::Dir(Dir {
                    perms: Permissions {
                        mode: Mode::RWXU | Mode::RGRP | Mode::XGRP | Mode::ROTH | Mode::WOTH,
                        userinfo: UserInfo { user: 0, group: 0 },
                    },
                    children_count: 0,
                }),
            )]
            .into_iter()
            .collect(),
        }
    }

    fn parent_and_entry(&self, path: &str, current_user: UserInfo) -> ParentAndEntry<&Dir, &Entry> {
        let mut real_components_seen = false;
        let mut collected = String::new();
        let mut parent_dir = None;
        for p in path.normalized_components()? {
            if p.is_empty() || p == ".." {
                // After normalization, these can only be at the start of the path, so can all be
                // ignored. We do an `assert` here mostly as a sanity check.
                assert!(!real_components_seen);
                continue;
            }
            // We have seen real components, should no longer see any empty or `/`s.
            real_components_seen = true;
            match self
                .entries
                .get_key_value(&collected)
                .ok_or(PathError::MissingComponent)?
            {
                (_, Entry::File(_)) => return Err(PathError::ComponentNotADirectory),
                (parent_path, Entry::Dir(dir)) => {
                    if !current_user.can_execute(&dir.perms) {
                        return Err(PathError::NoSearchPerms);
                    }
                    parent_dir = Some((parent_path.as_str(), dir));
                }
            }
            collected += "/";
            collected += p;
        }
        Ok((parent_dir, self.entries.get(&collected)))
    }
    fn parent_and_entry_mut(
        &mut self,
        path: &str,
        current_user: UserInfo,
    ) -> ParentAndEntry<&mut Dir, &mut Entry> {
        let mut real_components_seen = false;
        let mut collected = String::new();
        let mut parent_path = None;
        for p in path.normalized_components()? {
            if p.is_empty() || p == ".." {
                // After normalization, these can only be at the start of the path, so can all be
                // ignored. We do an `assert` here mostly as a sanity check.
                assert!(!real_components_seen);
                continue;
            }
            // We have seen real components, should no longer see any empty or `/`s.
            real_components_seen = true;
            match self
                .entries
                .get_mut(&collected)
                .ok_or(PathError::MissingComponent)?
            {
                Entry::File(_) => return Err(PathError::ComponentNotADirectory),
                Entry::Dir(dir) => {
                    if !current_user.can_execute(&dir.perms) {
                        return Err(PathError::NoSearchPerms);
                    }
                    parent_path = Some(collected.clone());
                }
            }
            collected += "/";
            collected += p;
        }
        if let Some(parent_path) = parent_path {
            let [parent_path_and_entry, main_path_and_entry] = self
                .entries
                .get_many_key_value_mut([&parent_path, &collected]);
            let (parent_path, parent_dir) = match parent_path_and_entry.unwrap() {
                (_, Entry::File(_)) => unreachable!(),
                (path, Entry::Dir(dir)) => (path, dir),
            };
            let main_entry = main_path_and_entry.map(|(_, e)| e);
            Ok((Some((parent_path, parent_dir)), main_entry))
        } else {
            Ok((None, self.entries.get_mut(&collected)))
        }
    }
}

enum Entry {
    File(File),
    Dir(Dir),
}

struct Dir {
    perms: Permissions,
    children_count: u32,
}

struct File {
    perms: Permissions,
    // TODO: Actual data
}

struct Permissions {
    mode: Mode,
    userinfo: UserInfo,
}

#[derive(Clone, Copy)]
struct UserInfo {
    user: u16,
    group: u16,
}

impl UserInfo {
    fn can_read(self, perms: &Permissions) -> bool {
        perms.can_read_by(self)
    }
    fn can_write(self, perms: &Permissions) -> bool {
        perms.can_write_by(self)
    }
    fn can_execute(self, perms: &Permissions) -> bool {
        perms.can_execute_by(self)
    }
}

impl Permissions {
    fn can_read_by(&self, current: UserInfo) -> bool {
        if self.userinfo.user == current.user {
            self.mode.contains(Mode::RUSR)
        } else if self.userinfo.group == current.group {
            self.mode.contains(Mode::RGRP)
        } else {
            self.mode.contains(Mode::ROTH)
        }
    }
    fn can_write_by(&self, current: UserInfo) -> bool {
        if self.userinfo.user == current.user {
            self.mode.contains(Mode::WUSR)
        } else if self.userinfo.group == current.group {
            self.mode.contains(Mode::WGRP)
        } else {
            self.mode.contains(Mode::WOTH)
        }
    }
    fn can_execute_by(&self, current: UserInfo) -> bool {
        if self.userinfo.user == current.user {
            self.mode.contains(Mode::XUSR)
        } else if self.userinfo.group == current.group {
            self.mode.contains(Mode::XGRP)
        } else {
            self.mode.contains(Mode::XOTH)
        }
    }
}
