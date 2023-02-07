use core::{arch::global_asm, cell::OnceCell};

use fdt::{standard_nodes::MemoryRegion, Fdt};
use log::trace;

use crate::{
    common::elf64::dynamic::{self, Dyn},
    drivers::serial::{
        pl011::{Config, Parity, Pl011},
        Serial, SerialLogger,
    },
    kernel::memory::Bitmap,
    label,
};

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

static mut PL011: OnceCell<SerialLogger<Pl011>> = OnceCell::new();

fn split_larger(region: MemoryRegion, start: u64, end: u64) -> MemoryRegion {
    trace!(
        "Splitting with region {:p} - {:p}",
        start as *const (),
        end as *const ()
    );
    let region_start = region.starting_address as u64;
    let region_end = region.starting_address.wrapping_add(region.size.unwrap()) as u64;
    if start >= region_start && end <= region_end {
        let left_region = MemoryRegion {
            starting_address: region_start as *const _,
            size: Some((end - region_start) as usize),
        };
        let right_region = MemoryRegion {
            starting_address: end as *const _,
            size: Some((region_end - end) as usize),
        };
        if left_region.size.unwrap() > right_region.size.unwrap() {
            left_region
        } else {
            right_region
        }
    } else if start >= region_start && start <= region_end {
        MemoryRegion {
            starting_address: region_start as *const _,
            size: Some((start - region_start) as usize),
        }
    } else if end >= region_start && end <= region_end {
        MemoryRegion {
            starting_address: end as *const _,
            size: Some((region_end - end) as usize),
        }
    } else {
        region
    }
}

#[no_mangle]
pub unsafe extern "C" fn init(dtb_ptr: *const u8) -> ! {
    let device_tree = Fdt::from_ptr(dtb_ptr).unwrap();

    trace!("Device tree: {:?}", device_tree);

    if let Some(stdout) = device_tree.chosen().stdout() {
        let ty = stdout.name.split('@').next().unwrap();
        match ty {
            "pl011" => {
                let mut serial = Pl011::new(
                    u64::from_str_radix(stdout.name.split('@').nth(1).unwrap(), 16).unwrap()
                        as *mut _,
                );
                serial
                    .init(Config {
                        baud_rate: 115_200,
                        clock_rate: 24_000_000,
                        parity: Parity::None,
                    })
                    .unwrap();
                // SAFETY: This function will never return, so for all intents and purposes, `logger` lives for `'static`.
                PL011
                    .get_or_init(|| SerialLogger::new(serial))
                    .set_logger()
                    .unwrap();
                trace!("Pl011 Initialized");
            }
            _ => unimplemented!("stdout type: {}", ty),
        }
    }

    let last_region = device_tree
        .memory()
        .regions()
        .last()
        .expect("There's no memory!");
    let last_region_end = last_region
        .starting_address
        .wrapping_add(last_region.size.unwrap());
    let first_region = device_tree
        .memory()
        .regions()
        .next()
        .expect("There's no memory!");
    let first_region_start = first_region.starting_address;
    let ram_size = last_region_end.wrapping_sub(first_region_start as usize) as usize;
    let bitmap_size = (ram_size / 4096) / 8;

    let mut bitmap_start = None;

    let kernel_start = label!(kernel_start) as u64;
    let kernel_end = label!(kernel_end) as u64;
    let dtb_end = dtb_ptr.wrapping_add(device_tree.total_size()) as u64;

    for mut region in device_tree.memory().regions() {
        trace!(
            "Memory region: {:p} - {:p}",
            region.starting_address,
            region.starting_address.wrapping_add(region.size.unwrap())
        );

        region = split_larger(region, kernel_start, kernel_end);
        region = split_larger(region, dtb_ptr as u64, dtb_end);

        for reservation in device_tree.memory_reservations() {
            region = split_larger(
                region,
                reservation.address() as u64,
                reservation.address().wrapping_add(reservation.size()) as u64,
            )
        }

        if region.size.unwrap() >= bitmap_size {
            bitmap_start = Some(region.starting_address);
        }
    }

    if bitmap_start.is_none() {
        panic!("No memory region large enough to hold the bitmap!");
    }

    trace!("Bitmap at {:p}", bitmap_start.unwrap());

    core::ptr::write_bytes(bitmap_start.unwrap() as *mut u8, 0xFF, bitmap_size);
    let mut bitmap = Bitmap::from_ptr(bitmap_start.unwrap() as *mut _, bitmap_size);

    for region in device_tree.memory().regions() {
        let start = region.starting_address as u64 / 4096;
        let end = region.starting_address.wrapping_add(region.size.unwrap()) as u64 / 4096;
        bitmap.clear_range(start as usize, end as usize);
    }

    bitmap.set_range(
        kernel_start as usize / 4096,
        (kernel_end + 4095) as usize / 4096,
    );
    bitmap.set_range(dtb_ptr as usize / 4096, (dtb_end + 4095) as usize / 4096);

    for reservation in device_tree.memory_reservations() {
        let start = reservation.address() as u64 / 4096;
        let end = (reservation.address().wrapping_add(reservation.size()) as u64 + 4095) / 4096;
        bitmap.set_range(start as usize, end as usize);
    }

    crate::kernel::memory::BITMAP.set(Some(bitmap));

    crate::main();
}
