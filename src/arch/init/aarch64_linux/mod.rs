use core::{arch::global_asm, mem::MaybeUninit, ptr::NonNull};

use fdt::{standard_nodes::MemoryRegion, Fdt};
use log::{error, info, trace};
use spin::Once;

use crate::{
    common::{
        elf64::dynamic::{self, Dyn},
        sizes::Size,
    },
    drivers::serial::{
        pl011::{Config, Parity, Pl011},
        Serial, SerialLogger,
    },
    kernel::memory::{
        physalloc::{Node, PhysAlloc},
        Bitmap,
    },
    label,
};

#[derive(Debug)]
struct MemRange {
    start: u64,
    end: u64,
}
impl MemRange {
    fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }
    fn size(&self) -> u64 {
        self.end - self.start
    }
    fn overlaps(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
    fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

struct InitRanges {
    pub ranges: heapless::Vec<MemRange, 128>,
}
impl InitRanges {
    fn new() -> Self {
        Self {
            ranges: heapless::Vec::new(),
        }
    }

    fn insert(&mut self, range: MemRange) {
        if self.ranges.iter().any(|other| other.overlaps(&range)) {
            panic!("Overlapping memory ranges");
        }

        let insertion_point = self
            .ranges
            .partition_point(|other| range.start < other.start);

        let mut inserted = false;

        if insertion_point > 0 {
            let prev = &mut self.ranges[insertion_point - 1];
            if prev.end == range.start {
                prev.end += range.size();
                inserted = true;
            }
        }

        if !self.ranges.is_empty() && insertion_point < self.ranges.len() - 1 {
            let next = &mut self.ranges[insertion_point + 1];
            if range.end == next.start {
                next.start = range.start;
                inserted = true;
            }
        }

        if inserted {
            return;
        }

        assert!(!self.ranges.is_full(), "too many memory ranges");
        self.ranges.insert(insertion_point, range);
    }

    fn remove(&mut self, range: MemRange) -> Option<MemRange> {
        let full = self.ranges.is_full();

        let (index, from_range) = self
            .ranges
            .iter_mut()
            .enumerate()
            .find(|(_, other)| other.contains(&range))?;

        if range.start == from_range.start && range.end == from_range.end {
            self.ranges.remove(index);
            return Some(range);
        }

        if range.start == from_range.start {
            from_range.start = range.end;
            return Some(range);
        }

        if range.end == from_range.end {
            from_range.end = range.start;
            return Some(range);
        }

        assert!(!full, "too many memory ranges");

        let new_range = MemRange::new(range.end, from_range.end);
        from_range.end = range.start;
        self.ranges.insert(index + 1, new_range);

        Some(range)
    }
}

global_asm!(include_str!("init.s"));

mod relocations {
    pub const NULL: u64 = 0;
    pub const WITHDRAWN: u64 = 256;

    pub const COPY: u64 = 1024;
    pub const GLOB_DAT: u64 = 1025;
    pub const JUMP_SLOT: u64 = 1026;
    pub const RELATIVE: u64 = 1027;
    pub const TLS_DPTREL64: u64 = 1028;
    pub const TLS_DTPMOD64: u64 = 1029;
    pub const TLS_TPREL64: u64 = 1030;
    pub const TLSDESC: u64 = 1031;
    pub const IRELATIVE: u64 = 1032;
}

#[no_mangle]
pub unsafe extern "C" fn relocate(base_addr: u64, dynamic_table: *const Dyn) -> bool {
    dynamic::relocate(base_addr as usize, dynamic_table, |rela| {
        match rela.r_type() {
            relocations::NULL => None,
            relocations::WITHDRAWN => None,

            relocations::COPY => unimplemented!("R_AARCH64_COPY"),
            relocations::GLOB_DAT => unimplemented!("R_AARCH64_GLOB_DAT"),
            relocations::JUMP_SLOT => unimplemented!("R_AARCH64_JUMP_SLOT"),
            relocations::RELATIVE => Some(base_addr.wrapping_add_signed(rela.addend)),
            relocations::TLS_DPTREL64 => panic!("This is a kernel, there is no thread-local storage (tried to do R_AARCH64_TLS_DPTREL64)."),
            relocations::TLS_DTPMOD64 => panic!("This is a kernel, there is no thread-local storage (tried to do R_AARCH64_TLS_DPTREL64)."),
            relocations::TLS_TPREL64 => panic!("This is a kernel, there is no thread-local storage (tried to do R_AARCH64_TLS_DPTREL64)."),
            relocations::TLSDESC => panic!("This is a kernel, there is no thread-local storage (tried to do R_AARCH64_TLS_DPTREL64)."),
            relocations::IRELATIVE => unimplemented!("R_AARCH64_IRELATIVE"),
            _ => panic!("Invalid relocation type: {}", rela.r_type()),
        }
    }).is_ok()
}

static mut PL011: Once<SerialLogger<Pl011>> = Once::new();

#[no_mangle]
pub unsafe extern "C" fn init(dtb_ptr: *const u8) -> ! {
    let device_tree = Fdt::from_ptr(dtb_ptr).unwrap();

    if let Some(stdout) = device_tree.chosen().stdout() {
        let Some(ty) = stdout.compatible() else {
            panic!("stdout is not compatible with any type");
        };
        let ty = ty.first();
        match ty {
            "arm,pl011" => {
                let mut serial =
                    Pl011::new(stdout.reg().unwrap().next().unwrap().starting_address as *mut _);
                serial
                    .init(Config {
                        baud_rate: 115_200,
                        clock_rate: 24_000_000,
                        parity: Parity::None,
                    })
                    .unwrap();
                PL011
                    .call_once(|| SerialLogger::new(serial))
                    .set_logger()
                    .unwrap();
                trace!("Pl011 Initialized");
            }
            _ => unimplemented!("stdout type: {}", ty),
        }
    }

    let mut ranges = InitRanges::new();

    for region in device_tree.memory().regions() {
        ranges.insert(MemRange::new(
            region.starting_address as u64,
            region.starting_address as u64 + region.size.unwrap() as u64,
        ));
    }

    for region in device_tree.memory_reservations() {
        ranges.remove(MemRange::new(
            region.address() as u64,
            region.address() as u64 + region.size() as u64,
        ));
    }

    ranges.remove(MemRange::new(
        dtb_ptr as u64,
        dtb_ptr as u64 + device_tree.total_size() as u64,
    ));

    ranges.remove(MemRange::new(
        label!(kernel_start) as u64,
        label!(kernel_end) as u64,
    ));

    for range in ranges.ranges.windows(2) {
        let start = (range[0].start + 4095) & !4095;
        let end = range[0].end & !4095;
        trace!(
            "Region: {:x} - {:x} ({})",
            start,
            end,
            Size(end as usize - start as usize)
        );
        for i in (start..end).step_by(4096) {
            let node = Node {
                next: NonNull::new((i + 4096) as *mut Node),
            };
            (i as *mut Node).write(node);
        }
        let next_start = (range[1].start + 4095) & !4095;
        ((end - 4096) as *mut Node).write(Node {
            next: NonNull::new(next_start as *mut Node),
        });
    }

    if let Some(last) = ranges.ranges.last() {
        let start = (last.start + 4095) & !4095;
        let end = last.end & !4095;
        trace!(
            "Region: {:x} - {:x} ({})",
            start,
            end,
            Size(end as usize - start as usize)
        );
        for i in (start..end).step_by(4096) {
            let node = Node {
                next: NonNull::new((i + 4096) as *mut Node),
            };
            (i as *mut Node).write(node);
        }
        ((end - 4096) as *mut Node).write(Node { next: None });
    }

    let head = || NonNull::new(((ranges.ranges.first()?.start + 4095) / 4096 * 4096) as *mut Node);
    let physalloc = PhysAlloc {
        free: head(),
        dirty: None,
    };

    trace!("Initialized physical allocator: {physalloc:?}");

    crate::main();
}
