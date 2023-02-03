use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_arch = "aarch64", feature = "linux"))] {
        mod aarch64_linux;
    }
}
