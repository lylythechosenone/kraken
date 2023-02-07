#[macro_export]
macro_rules! size_of {
    ($type:ty) => {
        ::core::mem::size_of::<$type>()
    };
}

#[macro_export]
macro_rules! align_of {
    ($type:ty) => {
        ::core::mem::align_of::<$type>()
    };
}

#[macro_export]
macro_rules! label {
    ($name:ident) => {
        $crate::label!($name: ())
    };
    (mut $name:ident) => {
        $crate::label!(mut $name: ())
    };
    ($name:ident: $type:ty) => {{
        #[allow(improper_ctypes)]
        extern "C" {
            pub static $name: $type;
        }
        unsafe { ::core::ptr::addr_of!($name) }
    }};
    (mut $sym:ident: $type:ty) => {{
        #[allow(improper_ctypes)]
        extern "C" {
            static $sym: $type;
        }
        unsafe { ::core::ptr::addr_of_mut!($sym) }
    }};
}
