use crate::kernel::{memory::address::PhysAddr, Process};

pub enum SyncException {
    FailedLoad,
    InvalidOpcode,
    FaultyDiv,
    IllegalFlop,
}

pub struct IntHandlers {
    page_fault: fn(PhysAddr, Process) -> bool,
    sync_exception: fn(PhysAddr, SyncException) -> bool,
    irq: fn(PhysAddr) -> bool,
}
impl IntHandlers {
    pub fn new(
        page_fault: fn(PhysAddr, Process) -> bool,
        sync_exception: fn(PhysAddr, SyncException) -> bool,
        irq: fn(PhysAddr) -> bool,
    ) -> IntHandlers {
        IntHandlers {
            page_fault,
            sync_exception,
            irq,
        }
    }
}
