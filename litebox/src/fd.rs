//! File descriptors used in LiteBox

/// A raw platform-specific file descriptor, coerced to fit into an `i32`.
pub type RawFd = i32;

/// An owned file descriptor.
///
/// This file descriptor **must** be consumed by a `close` operation, otherwise will panic at
/// run-time upon being dropped.
pub struct OwnedFd {
    fd: RawFd,
}

impl OwnedFd {
    pub(crate) fn new(fd: RawFd) -> Self {
        assert!(fd >= 0, "Cannot have a negative Raw FD. Got {fd}");
        Self { fd }
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.fd < 0
    }
    pub(crate) fn mark_as_closed(&mut self) {
        assert!(!self.is_closed());
        self.fd = -1;
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        if self.fd < 0 {
            // This has been closed out by a valid close operation
        } else {
            // The owned fd is dropped without being consumed by a `close` operation that has
            // properly marked it as being safely closed
            panic!("Un-closed OwnedFd ({}) being dropped", self.fd)
        }
    }
}

impl OwnedFd {
    /// Obtain the raw underlying platform-specific file-descriptor.
    pub fn as_raw_fd(&self) -> RawFd {
        if self.is_closed() {
            unreachable!(
                "As an invariant, close operations are supposed to consume the OwnedFd, \
                 thus no user should be able to invoke `as_raw_fd` on a closed OwnedFd."
            )
        }
        self.fd
    }
}
