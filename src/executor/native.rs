use std::ffi::{CStr, CString};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use crate::assets;
use crate::utils::{cpio::extract_rootfs_zstd, get_host_arch, mount::parse_mount_specs};

fn write_fd_bytes(path: &CStr, data: &[u8]) -> io::Result<()> {
    unsafe {
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY);
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let written = libc::write(fd, data.as_ptr() as *const libc::c_void, data.len());
        libc::close(fd);
        if written < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

pub fn run(mounts: &[String], cmd_args: &[String]) -> io::Result<i32> {
    let host_arch = get_host_arch()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "Unsupported host architecture"))?;
    let rootfs_bytes = assets::get_rootfs_bytes(host_arch);

    let rootfs_temp = extract_rootfs_zstd(rootfs_bytes)?;
    let rootfs_path = rootfs_temp.path();

    fs::create_dir_all(rootfs_path.join("proc"))?;
    fs::create_dir_all(rootfs_path.join("sys"))?;
    fs::create_dir_all(rootfs_path.join("dev"))?;

    let mount_specs = parse_mount_specs(mounts)?;
    let mut mount_pairs: Vec<(CString, CString)> = Vec::new();

    for spec in &mount_specs {
        let rel_guest = spec.guest_path.strip_prefix("/").unwrap_or(&spec.guest_path);
        let dest = rootfs_path.join(rel_guest);
        if spec.host_path.is_dir() {
            fs::create_dir_all(&dest)?;
        } else if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
            let _ = fs::File::create(&dest);
        }

        if let (Ok(host_c), Ok(dest_c)) = (
            CString::new(spec.host_path.as_os_str().as_bytes()),
            CString::new(dest.as_os_str().as_bytes()),
        ) {
            mount_pairs.push((host_c, dest_c));
        }
    }

    let is_root = unsafe { libc::geteuid() == 0 };

    let default_cmd = ["/bin/sh".to_string()];
    let effective_args = if cmd_args.is_empty() {
        &default_cmd[..]
    } else {
        cmd_args
    };

    let bin_name = &effective_args[0];
    let exec_target = if bin_name.starts_with('/') {
        bin_name.to_string()
    } else if rootfs_path.join("usr/bin").join(bin_name).exists() {
        format!("/usr/bin/{}", bin_name)
    } else if rootfs_path.join("bin").join(bin_name).exists() {
        format!("/bin/{}", bin_name)
    } else if rootfs_path.join("usr/sbin").join(bin_name).exists() {
        format!("/usr/sbin/{}", bin_name)
    } else if rootfs_path.join("sbin").join(bin_name).exists() {
        format!("/sbin/{}", bin_name)
    } else {
        format!("/bin/{}", bin_name)
    };
    let rest_args: Vec<String> = effective_args[1..].to_vec();

    let rootfs_c = CString::new(rootfs_path.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let proc_target_c = CString::new(rootfs_path.join("proc").as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let sys_target_c = CString::new(rootfs_path.join("sys").as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let dev_target_c = CString::new(rootfs_path.join("dev").as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let uid_map_bytes = format!("0 {} 1\n", uid).into_bytes();
    let gid_map_bytes = format!("0 {} 1\n", gid).into_bytes();

    let mut command = Command::new(&exec_target);
    for arg in &rest_args {
        command.arg(arg);
    }

    unsafe {
        command.pre_exec(move || {
            if is_root {
                if libc::unshare(libc::CLONE_NEWNS) != 0 {
                    return Err(io::Error::last_os_error());
                }
                let _ = libc::mount(
                    std::ptr::null(),
                    c"/".as_ptr(),
                    std::ptr::null(),
                    libc::MS_REC | libc::MS_PRIVATE,
                    std::ptr::null(),
                );
            } else {
                if libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) != 0 {
                    return Err(io::Error::last_os_error());
                }
                let _ = write_fd_bytes(c"/proc/self/setgroups", b"deny");
                let _ = write_fd_bytes(c"/proc/self/uid_map", &uid_map_bytes);
                let _ = write_fd_bytes(c"/proc/self/gid_map", &gid_map_bytes);
                let _ = libc::mount(
                    std::ptr::null(),
                    c"/".as_ptr(),
                    std::ptr::null(),
                    libc::MS_REC | libc::MS_PRIVATE,
                    std::ptr::null(),
                );
            }

            let _ = libc::mount(
                c"proc".as_ptr(),
                proc_target_c.as_ptr(),
                c"proc".as_ptr(),
                libc::MS_NOSUID | libc::MS_NOEXEC | libc::MS_NODEV,
                std::ptr::null(),
            );
            let _ = libc::mount(
                c"sysfs".as_ptr(),
                sys_target_c.as_ptr(),
                c"sysfs".as_ptr(),
                libc::MS_NOSUID | libc::MS_NOEXEC | libc::MS_NODEV,
                std::ptr::null(),
            );
            let _ = libc::mount(
                c"/dev".as_ptr(),
                dev_target_c.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND | libc::MS_REC,
                std::ptr::null(),
            );

            for (host_c, dest_c) in &mount_pairs {
                let _ = libc::mount(
                    host_c.as_ptr(),
                    dest_c.as_ptr(),
                    std::ptr::null(),
                    libc::MS_BIND | libc::MS_REC,
                    std::ptr::null(),
                );
            }

            if libc::chroot(rootfs_c.as_ptr()) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::chdir(c"/".as_ptr()) != 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        });
    }

    command.env("LD_LIBRARY_PATH", "/lib:/usr/lib")
           .env("PATH", "/bin:/usr/bin:/sbin:/usr/sbin")
           .env("HOME", "/root")
           .env("USER", "root")
           .env("TERM", "xterm-256color");

    command.stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());

    super::spawn_and_wait(command)
}
