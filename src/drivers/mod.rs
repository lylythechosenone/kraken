pub mod serial;

pub trait MmioDevice {
    fn pointer(&self) -> *mut u8;
    fn get_register(&self, register: usize) -> *mut u8 {
        self.pointer().wrapping_add(register)
    }
    fn get_register_byte(&self, register: usize, offset: usize) -> *mut u8 {
        self.get_register(register).wrapping_add(offset)
    }
    fn get_register_64(&self, register: usize) -> *mut u64 {
        self.get_register(register) as *mut u64
    }
    fn get_register_32(&self, register: usize) -> *mut u32 {
        self.get_register(register) as *mut u32
    }
    fn get_register_16(&self, register: usize) -> *mut u16 {
        self.get_register(register) as *mut u16
    }
    fn get_register_8(&self, register: usize) -> *mut u8 {
        self.get_register(register) as *mut u8
    }

    fn write_register_byte(&mut self, register: usize, offset: usize, value: u8) {
        unsafe {
            core::ptr::write_volatile(self.get_register_byte(register, offset), value);
        }
    }
    fn write_register_64(&mut self, register: usize, value: u64) {
        unsafe {
            core::ptr::write_volatile(self.get_register_64(register), value);
        }
    }
    fn write_register_32(&mut self, register: usize, value: u32) {
        unsafe {
            core::ptr::write_volatile(self.get_register_32(register), value);
        }
    }
    fn write_register_16(&mut self, register: usize, value: u16) {
        unsafe {
            core::ptr::write_volatile(self.get_register_16(register), value);
        }
    }
    fn write_register_8(&mut self, register: usize, value: u8) {
        unsafe {
            core::ptr::write_volatile(self.get_register_8(register), value);
        }
    }

    fn read_register_byte(&self, register: usize, offset: usize) -> u8 {
        unsafe { core::ptr::read_volatile(self.get_register_byte(register, offset)) }
    }
    fn read_register_64(&self, register: usize) -> u64 {
        unsafe { core::ptr::read_volatile(self.get_register_64(register)) }
    }
    fn read_register_32(&self, register: usize) -> u32 {
        unsafe { core::ptr::read_volatile(self.get_register_32(register)) }
    }
    fn read_register_16(&self, register: usize) -> u16 {
        unsafe { core::ptr::read_volatile(self.get_register_16(register)) }
    }
    fn read_register_8(&self, register: usize) -> u8 {
        unsafe { core::ptr::read_volatile(self.get_register_8(register)) }
    }
}
