//! Handling the allocation of local ports

use core::num::{NonZeroU16, NonZeroU64};

use hashbrown::HashSet;
use thiserror::Error;

use crate::utilities::rng::FastRng;

/// An allocator for local ports, making sure that no already-allocated ports are given out
pub(crate) struct LocalPortAllocator {
    allocated: HashSet<NonZeroU16>,
    rng: FastRng,
}

impl Default for LocalPortAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalPortAllocator {
    /// Sets up a new local port allocator
    pub(crate) fn new() -> Self {
        Self {
            allocated: HashSet::new(),
            rng: FastRng::new_from_seed(NonZeroU64::new(0x13374a4159421337).unwrap()),
        }
    }

    /// Allocate a new ephemeral local port (i.e., port in the range 49152 and 65535)
    pub(crate) fn ephemeral_port(&mut self) -> Result<LocalPort, LocalPortAllocationError> {
        for _ in 0..100 {
            let port =
                NonZeroU16::new(u16::try_from(self.rng.next_in_range_u32(49152..65536)).unwrap())
                    .unwrap();
            if let Ok(local_port) = self.specific_port(port) {
                return Ok(local_port);
            }
        }
        // If we haven't yet found a port after 100 tries, it is highly likely lots of ports are
        // already in use, so we should start looking over them one by one
        for port in 49152..=65535 {
            let port = NonZeroU16::new(port).unwrap();
            if let Ok(local_port) = self.specific_port(port) {
                return Ok(local_port);
            }
        }
        // If we _still_ haven't found any, then we have run out of ports to give out
        Err(LocalPortAllocationError::NoAvailableFreePorts)
    }

    /// Allocate a specific local port, if available
    pub(crate) fn specific_port(
        &mut self,
        port: NonZeroU16,
    ) -> Result<LocalPort, LocalPortAllocationError> {
        if self.allocated.insert(port) {
            Ok(LocalPort { port })
        } else {
            Err(LocalPortAllocationError::AlreadyInUse(port.get()))
        }
    }

    /// Increments the ref-count for a local port, producing a new `LocalPort` token to be used
    #[must_use]
    pub(crate) fn allocate_same_local_port(&mut self, port: &LocalPort) -> LocalPort {
        // TODO(jayb): Definitely have to rethink this entire module now that I want this particular
        // interface here.
        todo!()
    }

    /// Marks a [`LocalPort`] as available again, consuming it
    pub(crate) fn deallocate(&mut self, port: LocalPort) {
        let was_removed = self.allocated.remove(&port.port);
        // As an invariant, the only production of `LocalPort` can happen from here, thus it should
        // be impossible to have a `LocalPort` containing a non-allocated spot.
        assert!(was_removed);
    }
}

/// A token expressing ownership over a specific local port.
///
/// Explicitly not cloneable/copyable.
pub(crate) struct LocalPort {
    port: NonZeroU16,
}

impl LocalPort {
    pub(crate) fn port(&self) -> u16 {
        self.port.get()
    }
}

/// Errors that could be returned when allocating a [`LocalPort`]
#[derive(Debug, Error)]
pub enum LocalPortAllocationError {
    #[error("Port {0} is already in use")]
    AlreadyInUse(u16),
    #[error("No free ports are available")]
    NoAvailableFreePorts,
}
