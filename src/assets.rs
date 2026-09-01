pub static ROOTFS_X86_64: &[u8] = include_bytes!("../assets/rootfs-x86_64.cpio.zst");
pub static ROOTFS_AARCH64: &[u8] = include_bytes!("../assets/rootfs-aarch64.cpio.zst");
pub static KERNEL_X86_64: &[u8] = include_bytes!("../assets/kernel-x86_64.zst");
pub static KERNEL_AARCH64: &[u8] = include_bytes!("../assets/kernel-aarch64.zst");
pub static PC_BIOS: &[u8] = include_bytes!("../assets/pc-bios.tar.zst");

#[cfg(target_arch = "x86_64")]
pub static QEMU_USER_X86_64: &[u8] = include_bytes!("../assets/x86_64/qemu-x86_64.zst");
#[cfg(target_arch = "x86_64")]
pub static QEMU_USER_AARCH64: &[u8] = include_bytes!("../assets/x86_64/qemu-aarch64.zst");
#[cfg(target_arch = "x86_64")]
pub static QEMU_SYSTEM_X86_64: &[u8] = include_bytes!("../assets/x86_64/qemu-system-x86_64.zst");
#[cfg(target_arch = "x86_64")]
pub static QEMU_SYSTEM_AARCH64: &[u8] = include_bytes!("../assets/x86_64/qemu-system-aarch64.zst");

#[cfg(target_arch = "aarch64")]
pub static QEMU_USER_X86_64: &[u8] = include_bytes!("../assets/aarch64/qemu-x86_64.zst");
#[cfg(target_arch = "aarch64")]
pub static QEMU_USER_AARCH64: &[u8] = include_bytes!("../assets/aarch64/qemu-aarch64.zst");
#[cfg(target_arch = "aarch64")]
pub static QEMU_SYSTEM_X86_64: &[u8] = include_bytes!("../assets/aarch64/qemu-system-x86_64.zst");
#[cfg(target_arch = "aarch64")]
pub static QEMU_SYSTEM_AARCH64: &[u8] = include_bytes!("../assets/aarch64/qemu-system-aarch64.zst");

use crate::cli::Architecture;

pub fn get_rootfs_bytes(arch: Architecture) -> &'static [u8] {
    match arch {
        Architecture::X86_64 => ROOTFS_X86_64,
        Architecture::Aarch64 => ROOTFS_AARCH64,
    }
}

pub fn get_kernel_bytes(arch: Architecture) -> &'static [u8] {
    match arch {
        Architecture::X86_64 => KERNEL_X86_64,
        Architecture::Aarch64 => KERNEL_AARCH64,
    }
}

pub fn get_qemu_user_bytes(arch: Architecture) -> &'static [u8] {
    match arch {
        Architecture::X86_64 => QEMU_USER_X86_64,
        Architecture::Aarch64 => QEMU_USER_AARCH64,
    }
}

pub fn get_qemu_system_bytes(arch: Architecture) -> &'static [u8] {
    match arch {
        Architecture::X86_64 => QEMU_SYSTEM_X86_64,
        Architecture::Aarch64 => QEMU_SYSTEM_AARCH64,
    }
}
