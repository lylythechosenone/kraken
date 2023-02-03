#[macro_export]
macro_rules! size_of {
    ($type:ty) => {
        ::core::mem::size_of::<$type>()
    };
}
