pub mod cpio;
pub mod cpu;
pub mod memfd;
pub mod mount;

use crate::cli::Architecture;

pub fn get_host_arch() -> Option<Architecture> {
    #[cfg(target_arch = "x86_64")]
    return Some(Architecture::X86_64);

    #[cfg(target_arch = "aarch64")]
    return Some(Architecture::Aarch64);

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return None;
}
