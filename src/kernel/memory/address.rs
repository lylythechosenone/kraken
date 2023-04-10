use core::marker::PhantomData;

mod sealed {
    pub trait AddrSpace {
        type Pointer<T>: PartialEq + PartialOrd + Copy;

        fn cast_ptr<T, U>(ptr: Self::Pointer<T>) -> Self::Pointer<U>;
    }
}

pub struct Virtual;
impl sealed::AddrSpace for Virtual {
    type Pointer<T> = *mut T;

    fn cast_ptr<T, U>(ptr: *mut T) -> *mut U {
        ptr as *mut U
    }
}

pub struct Physical;
impl sealed::AddrSpace for Physical {
    type Pointer<T> = usize;

    fn cast_ptr<T, U>(ptr: usize) -> usize {
        ptr
    }
}

pub struct Address<Space: sealed::AddrSpace> {
    address: Space::Pointer<()>,
    _phantom: PhantomData<Space>,
}
impl<Space: sealed::AddrSpace> Address<Space> {
    pub const fn new(address: Space::Pointer<()>) -> Self {
        Self {
            address,
            _phantom: PhantomData,
        }
    }
    pub const fn get(&self) -> Space::Pointer<()> {
        self.address
    }

    pub fn into_ptr<T>(self) -> Pointer<T, Space> {
        Pointer::new(Space::cast_ptr(self.address))
    }
}
impl<Space: sealed::AddrSpace> PartialEq for Address<Space> {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}
impl<Space: sealed::AddrSpace> PartialOrd for Address<Space> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.address.partial_cmp(&other.address)
    }
}
impl<Space: sealed::AddrSpace> Clone for Address<Space> {
    fn clone(&self) -> Self {
        Self::new(self.address)
    }
}
impl<Space: sealed::AddrSpace> Copy for Address<Space> {}

pub type VirtAddr = Address<Virtual>;
pub type PhysAddr = Address<Physical>;

impl VirtAddr {
    pub fn to_phys_offset(self, offset: usize) -> PhysAddr {
        PhysAddr::new(self.get() as usize - offset)
    }
}
impl PhysAddr {
    pub const fn to_virt_offset(self, offset: usize) -> VirtAddr {
        VirtAddr::new((self.get() + offset) as *mut _)
    }
}

pub struct Pointer<T, Space: sealed::AddrSpace> {
    pointer: Space::Pointer<T>,
    _phantom: PhantomData<T>,
}
impl<T, Space: sealed::AddrSpace> Pointer<T, Space> {
    pub const fn new(pointer: Space::Pointer<T>) -> Self {
        Self {
            pointer,
            _phantom: PhantomData,
        }
    }
    pub const fn get(&self) -> Space::Pointer<T> {
        self.pointer
    }

    pub fn into_address(self) -> Address<Space> {
        Address::new(Space::cast_ptr(self.pointer))
    }
    pub fn cast<U>(self) -> Pointer<U, Space> {
        Pointer::new(Space::cast_ptr(self.pointer))
    }
}
impl<T, Space: sealed::AddrSpace> PartialEq for Pointer<T, Space> {
    fn eq(&self, other: &Self) -> bool {
        self.pointer == other.pointer
    }
}
impl<T, Space: sealed::AddrSpace> PartialOrd for Pointer<T, Space> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.pointer.partial_cmp(&other.pointer)
    }
}
impl<T, Space: sealed::AddrSpace> Clone for Pointer<T, Space> {
    fn clone(&self) -> Self {
        Self::new(self.pointer)
    }
}
impl<T, Space: sealed::AddrSpace> Copy for Pointer<T, Space> {}

pub type VirtPtr<T> = Pointer<T, Virtual>;
pub type PhysPtr<T> = Pointer<T, Physical>;

impl<T> VirtPtr<T> {
    pub fn to_phys_offset(self, offset: usize) -> PhysPtr<T> {
        PhysPtr::new(self.get() as usize - offset)
    }
    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut *self.get()
    }
    pub unsafe fn as_ref(&self) -> &T {
        &*self.get()
    }
}
impl<T> PhysPtr<T> {
    pub const fn to_virt_offset(self, offset: usize) -> VirtPtr<T> {
        VirtPtr::new((self.get() + offset) as *mut _)
    }
}
