pub struct CpuInfo;
impl CpuInfo {
    pub fn num_cpus() -> usize {
        unsafe { crate::backend::num_cpus() }
    }
    pub fn cpu_id() -> usize {
        unsafe { crate::backend::cpu_id() }
    }
}
