//! Possible errors from [`FileSystem`]

#[expect(
    unused_imports,
    reason = "used for doc string links to work out, but not for code"
)]
use super::FileSystem;

use thiserror::Error;

/// Possible errors from [`FileSystem::foo`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum FooError {}

/// Possible errors from [`FileSystem::open`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum OpenError {}

/// Possible errors from [`FileSystem::close`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum CloseError {}

/// Possible errors from [`FileSystem::read`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ReadError {}

/// Possible errors from [`FileSystem::write`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum WriteError {}

/// Possible errors from [`FileSystem::chmod`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ChmodError {}

/// Possible errors from [`FileSystem::unlink`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum UnlinkError {}

/// Possible errors from [`FileSystem::mkdir`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum MkdirError {}

/// Possible errors from [`FileSystem::rmdir`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum RmdirError {}
