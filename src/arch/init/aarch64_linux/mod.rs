use core::{arch::global_asm, cell::OnceCell};

use fdt::Fdt;
use log::info;

use crate::{
    common::elf64::dynamic::{self, Dyn},
    drivers::serial::{
        pl011::{Config, Parity, Pl011},
        Serial, SerialLogger,
    },
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

#[no_mangle]
pub unsafe extern "C" fn init(device_tree: *const u8) -> ! {
    let device_tree = Fdt::from_ptr(device_tree).unwrap();

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
                info!("Pl011 Initialized");
            }
            _ => unimplemented!("stdout type: {}", ty),
        }
    }

    crate::main();
}
