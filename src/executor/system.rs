use std::io::{self, Cursor};
use std::process::{Command, Stdio};
use base64::Engine;
use crate::assets;
use crate::cli::{AccelMode, Architecture};
use crate::utils::{
    cpio::create_overlay_cpio,
    get_host_arch,
    memfd::{create_executable_from_zstd, write_bytes_to_temp_file},
    mount::parse_mount_specs,
};

pub struct SystemOptions<'a> {
    pub arch: Architecture,
    pub accel_mode: AccelMode,
    pub mounts: &'a [String],
    pub no_cpu_topo: bool,
    pub cpu_part: Option<usize>,
    pub cpu: Option<&'a str>,
    pub mem: Option<&'a str>,
    pub qemu_overrides: &'a [String],
    pub cmd_args: &'a [String],
}

fn is_singleton_opt(opt: &str) -> bool {
    matches!(
        opt,
        "-m" | "-smp" | "-accel" | "-M" | "-machine" | "-cpu" | "-L" | "-kernel" | "-initrd" | "-append"
    )
}

pub fn run(opts: SystemOptions<'_>) -> io::Result<i32> {
    let host_arch = get_host_arch();
    let arch = opts.arch;
    let accel_mode = opts.accel_mode;

    let kernel_bytes = assets::get_kernel_bytes(arch);
    let rootfs_bytes = assets::get_rootfs_bytes(arch);
    let qemu_system_bytes = assets::get_qemu_system_bytes(arch);

    let mount_specs = parse_mount_specs(opts.mounts)?;

    let qemu_exec = create_executable_from_zstd(&format!("qemu-system-{}", arch), qemu_system_bytes)?;
    let (_kernel_dir, kernel_path) = write_bytes_to_temp_file(&format!("kernel-{}", arch), kernel_bytes)?;
    let (_rootfs_dir, rootfs_path) = if mount_specs.is_empty() {
        write_bytes_to_temp_file(&format!("rootfs-{}.cpio", arch), rootfs_bytes)?
    } else {
        let mut base_cpio = zstd::decode_all(Cursor::new(rootfs_bytes))?;
        if let Some(trailer_offset) = crate::utils::cpio::find_cpio_trailer_offset(&base_cpio) {
            base_cpio.truncate(trailer_offset);
        }
        let overlay_cpio = create_overlay_cpio(&mount_specs)?;
        base_cpio.extend_from_slice(&overlay_cpio);
        crate::utils::memfd::write_raw_bytes_to_temp_file(&format!("rootfs-{}.cpio", arch), &base_cpio)?
    };
    let bios_guard = crate::utils::memfd::extract_tar_zstd_to_temp_dir("pc-bios", assets::PC_BIOS)?;

    if accel_mode == AccelMode::Kvm {
        let is_kvm_available = host_arch == Some(arch)
            && std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/kvm")
                .is_ok();
        if !is_kvm_available {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("KVM acceleration is not accessible for {} on this host. Check /dev/kvm permissions or use 'tcg'", arch),
            ));
        }
    }

    let default_accel = accel_mode.as_str();

    let mut selected_cluster: Option<crate::utils::cpu::CpuCluster> = None;
    if arch == Architecture::Aarch64 && accel_mode == AccelMode::Kvm {
        let clusters = crate::utils::cpu::detect_arm_clusters();
        if clusters.len() > 1 {
            selected_cluster = crate::utils::cpu::select_arm_cluster(&clusters, opts.no_cpu_topo, opts.cpu_part)?;
        } else if clusters.len() == 1 && !opts.no_cpu_topo {
            selected_cluster = Some(clusters[0].clone());
        }
    }

    let smp_count_opt: Option<String> = if let Some(c) = opts.cpu {
        if c == "0" {
            None
        } else {
            Some(c.to_string())
        }
    } else if let Some(ref cluster) = selected_cluster {
        Some(cluster.cores.len().to_string())
    } else {
        let num_cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
        if num_cpus > 0 {
            Some(num_cpus.to_string())
        } else {
            Some("2".to_string())
        }
    };

    let cpu_model = if default_accel == "kvm" {
        "host"
    } else {
        "max"
    };

    let mem_size_opt: Option<&str> = match opts.mem {
        Some("0") => None,
        Some(m) => Some(m),
        None => Some("1024M"),
    };

    let mut qemu_kv_args: Vec<(String, String)> = Vec::new();

    let set_kv_arg = |args: &mut Vec<(String, String)>, key: &str, val: &str| {
        if is_singleton_opt(key) {
            if let Some(pos) = args.iter().position(|(k, _)| k == key) {
                args[pos] = (key.to_string(), val.to_string());
                return;
            }
        }
        args.push((key.to_string(), val.to_string()));
    };

    match arch {
        Architecture::X86_64 => {
            set_kv_arg(&mut qemu_kv_args, "-M", "microvm,isa-serial=on,rtc=on");
        }
        Architecture::Aarch64 => {
            set_kv_arg(&mut qemu_kv_args, "-M", "virt");
        }
    }

    set_kv_arg(&mut qemu_kv_args, "-cpu", cpu_model);
    set_kv_arg(&mut qemu_kv_args, "-accel", default_accel);
    if let Some(ref smp) = smp_count_opt {
        set_kv_arg(&mut qemu_kv_args, "-smp", smp);
    }
    if let Some(mem) = mem_size_opt {
        set_kv_arg(&mut qemu_kv_args, "-m", mem);
    }
    let bios_dir_str = bios_guard.path().to_string_lossy();
    set_kv_arg(&mut qemu_kv_args, "-L", &bios_dir_str);

    let mut flat_qemu_overrides: Vec<String> = Vec::new();
    for item in opts.qemu_overrides {
        for part in item.split_whitespace() {
            flat_qemu_overrides.push(part.to_string());
        }
    }

    let mut extra_flags: Vec<String> = Vec::new();
    let mut i = 0;
    while i < flat_qemu_overrides.len() {
        let flag = &flat_qemu_overrides[i];
        if flag == "--" {
            i += 1;
            continue;
        }
        if flag.starts_with('-') && i + 1 < flat_qemu_overrides.len() && !flat_qemu_overrides[i + 1].starts_with('-') && flat_qemu_overrides[i + 1] != "--" {
            set_kv_arg(&mut qemu_kv_args, flag, &flat_qemu_overrides[i + 1]);
            i += 2;
        } else {
            extra_flags.push(flag.clone());
            i += 1;
        }
    }

    let mut kernel_cmdline = match arch {
        Architecture::Aarch64 => "console=ttyAMA0,115200 quiet panic=-1".to_string(),
        Architecture::X86_64 => "console=ttyS0 quiet panic=-1".to_string(),
    };

    if !opts.cmd_args.is_empty() {
        let cmd_quoted: Vec<String> = opts.cmd_args
            .iter()
            .map(|arg| format!("'{}'", arg.replace('\'', "'\\''")))
            .collect();
        let cmd_joined = cmd_quoted.join(" ");
        let b64 = base64::engine::general_purpose::STANDARD.encode(cmd_joined.as_bytes());
        kernel_cmdline.push_str(&format!(" qemucmd64={}", b64));
    }

    let mut command = Command::new(qemu_exec.path());

    for (key, val) in &qemu_kv_args {
        command.arg(key).arg(val);
    }
    for flag in &extra_flags {
        command.arg(flag);
    }

    command.arg("-kernel").arg(&kernel_path);
    command.arg("-initrd").arg(&rootfs_path);
    command.arg("-append").arg(&kernel_cmdline);
    command.arg("-nographic");
    command.arg("-serial").arg("stdio");
    command.arg("-monitor").arg("none");
    command.arg("-no-reboot");

    command.stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let pinned_cores = selected_cluster.map(|c| c.cores);
        unsafe {
            command.pre_exec(move || {
                if let Some(ref cores) = pinned_cores {
                    let mut set: libc::cpu_set_t = std::mem::zeroed();
                    libc::CPU_ZERO(&mut set);
                    for &c in cores {
                        libc::CPU_SET(c, &mut set);
                    }
                    let _ = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
                }
                Ok(())
            });
        }
    }

    super::spawn_and_wait_guest(command, opts.cmd_args.is_empty())
}
