use crate::size_of;

/// Dynamic linking information. See https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-42444.html for more info.

/// A dynamic entry, found in the `.dynamic` section, under the `_DYNAMIC` label.
#[repr(C)]
pub struct Dyn {
    tag: u64,
    val: u64,
}

const DYN_NULL: u64 = 0x00;
const DYN_RELA: u64 = 0x07;
const DYN_RELA_SIZE: u64 = 0x08;
const DYN_ENTRY_SIZE: u64 = 0x09;

/// Relocation via addend.
#[repr(C)]
pub struct Rela {
    /// The relocation's offset from the start of the executable
    pub offset: u64,
    /// Unused. See https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-54839.html for more info.
    pub info: u64,
    /// The amount to add to the start of the executable
    pub addend: i64,
}
impl Rela {
    pub fn r_type(&self) -> u64 {
        self.info & 0xffffffff
    }
}

/// An invalid relocation error. Only 2 possibilities exist:
///
/// 1. a table address and/or entry size exist, but a table size does not exist
/// 2. the entry size is incorrect
#[derive(Debug)]
pub struct InvalidRelocation;

/// Self-relocate the current executable.
///
/// ## Parameters
/// - `base_addr`: The base address of the executable
/// - `dynamic_table`: The dynamic table, found under the `_DYNAMIC` label
/// - `pred`: A predicate to apply to the relocation. It should return the value to be written to the relocation's target.
pub fn relocate(
    base_addr: usize,
    mut dynamic_table: *const Dyn,
    mut pred: impl FnMut(&Rela) -> Option<u64>,
) -> Result<(), InvalidRelocation> {
    let mut table_addr = None;
    let mut table_size = None;
    let mut entry_size = None;

    loop {
        let entry = unsafe { dynamic_table.read() };

        match entry.tag {
            DYN_NULL => break,
            DYN_RELA => table_addr = Some(entry.val),
            DYN_RELA_SIZE => table_size = Some(entry.val),
            DYN_ENTRY_SIZE => entry_size = Some(entry.val),
            _ => {}
        }

        dynamic_table = unsafe { dynamic_table.add(1) };
    }

    if table_addr.is_none() && entry_size.is_none() {
        return Ok(());
    }

    let Some(table_addr) = table_addr else {
        return Err(InvalidRelocation)
    };
    let Some(table_size) = table_size else {
        return Err(InvalidRelocation)
    };
    let Some(entry_size) = entry_size else {
        return Err(InvalidRelocation)
    };

    if entry_size != size_of!(Rela) as u64 {
        return Err(InvalidRelocation);
    }

    let data = base_addr.saturating_add(table_addr as usize) as *const Rela;
    let len = table_size / entry_size;
    let table = unsafe { core::slice::from_raw_parts(data, len as usize) };

    for relocation in table {
        let Some(value) = pred(relocation) else {
            continue;
        };
        let target = base_addr.saturating_add(relocation.offset as usize);
        unsafe { *(target as *mut usize) = value as usize };
    }
    Ok(())
}
