use core::arch::global_asm;

use crate::common::elf64::dynamic::{relocate, Dyn};

global_asm!(include_str!("aarch64_linux/init.s"));

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
pub extern "C" fn init(_device_tree: *const (), base_addr: u64, dynamic_table: *const Dyn) -> ! {
    relocate(base_addr as usize, dynamic_table, |rela| {
        match rela.r_type() {
            relocations::NULL => None,
            relocations::WITHDRAWN => None,

            relocations::COPY => unimplemented!("R_AARCH64_COPY"),
            relocations::GLOB_DAT => unimplemented!("R_AARCH64_GLOB_DAT"),
            relocations::JUMP_SLOT => unimplemented!("R_AARCH64_JUMP_SLOT"),
            relocations::RELATIVE => Some(base_addr.saturating_add_signed(rela.addend)),
            relocations::TLS_DPTREL64 => panic!("This is a kernel, there is no thread-local storage (tried to do R_AARCH64_DPTREL64)."),
            relocations::TLS_DTPMOD64 => todo!(),
            relocations::TLS_TPREL64 => todo!(),
            relocations::TLSDESC => todo!(),
            relocations::IRELATIVE => todo!(),
            _ => panic!("Invalid relocation type: {}", rela.r_type()),
        }
    })
    .unwrap();
    todo!()
}
