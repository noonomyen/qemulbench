use std::io;
use std::process::{Command, Stdio};
use crate::assets;
use crate::cli::Architecture;
use crate::utils::{
    cpio::extract_rootfs_zstd,
    memfd::create_executable_from_zstd,
    mount::{apply_user_symlinks, parse_mount_specs},
};

pub fn run(arch: Architecture, mounts: &[String], cmd_args: &[String]) -> io::Result<i32> {
    let rootfs_bytes = assets::get_rootfs_bytes(arch);
    let qemu_user_bytes = assets::get_qemu_user_bytes(arch);

    let rootfs_temp = extract_rootfs_zstd(rootfs_bytes)?;
    let rootfs_path = rootfs_temp.path();

    let mount_specs = parse_mount_specs(mounts)?;
    apply_user_symlinks(&mount_specs, rootfs_path)?;

    let qemu_exec = create_executable_from_zstd(&format!("qemu-{}", arch), qemu_user_bytes)?;

    let default_cmd = ["/bin/sh".to_string()];
    let effective_args = if cmd_args.is_empty() {
        &default_cmd[..]
    } else {
        cmd_args
    };

    let bin_name = &effective_args[0];
    let candidate_bin = if bin_name.starts_with('/') {
        rootfs_path.join(bin_name.trim_start_matches('/'))
    } else if rootfs_path.join("usr/bin").join(bin_name).exists() {
        rootfs_path.join("usr/bin").join(bin_name)
    } else if rootfs_path.join("bin").join(bin_name).exists() {
        rootfs_path.join("bin").join(bin_name)
    } else if rootfs_path.join("usr/sbin").join(bin_name).exists() {
        rootfs_path.join("usr/sbin").join(bin_name)
    } else if rootfs_path.join("sbin").join(bin_name).exists() {
        rootfs_path.join("sbin").join(bin_name)
    } else {
        rootfs_path.join("bin").join(bin_name)
    };

    let mut target_bin = candidate_bin;
    let mut symlink_depth = 0;
    while target_bin.is_symlink() && symlink_depth < 16 {
        symlink_depth += 1;
        if let Ok(target) = std::fs::read_link(&target_bin) {
            target_bin = if target.is_absolute() {
                rootfs_path.join(target.strip_prefix("/").unwrap_or(&target))
            } else {
                target_bin.parent().unwrap_or(rootfs_path).join(target)
            };
        } else {
            break;
        }
    }

    let mut command = Command::new(qemu_exec.path());
    command.arg("-L").arg(rootfs_path);
    command.arg("-0").arg(bin_name);
    command.arg(&target_bin);

    for arg in &effective_args[1..] {
        command.arg(arg);
    }

    command.stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());

    super::spawn_and_wait(command)
}
