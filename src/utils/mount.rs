use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MountSpec {
    pub host_path: PathBuf,
    pub guest_path: PathBuf,
}

impl MountSpec {
    pub fn parse(raw: &str, index: usize) -> io::Result<Self> {
        let (host_str, guest_str) = if let Some(pos) = raw.find(':') {
            (&raw[..pos], &raw[pos + 1..])
        } else {
            (raw, "")
        };

        if host_str.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Mount host path cannot be empty",
            ));
        }

        let host_path = fs::canonicalize(host_str).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to resolve host mount path '{}': {}", host_str, e),
            )
        })?;

        let guest_path = if guest_str.is_empty() {
            if index == 0 {
                PathBuf::from("/mnt/host")
            } else {
                PathBuf::from(format!("/mnt/host{}", index))
            }
        } else {
            let p = PathBuf::from(guest_str);
            if !p.is_absolute() {
                PathBuf::from("/").join(p)
            } else {
                p
            }
        };

        Ok(Self {
            host_path,
            guest_path,
        })
    }
}

pub fn parse_mount_specs(raw_mounts: &[String]) -> io::Result<Vec<MountSpec>> {
    raw_mounts
        .iter()
        .enumerate()
        .map(|(i, s)| MountSpec::parse(s, i))
        .collect()
}

pub fn apply_user_symlinks(mounts: &[MountSpec], rootfs_dir: &Path) -> io::Result<()> {
    for spec in mounts {
        let rel_guest = spec.guest_path.strip_prefix("/").unwrap_or(&spec.guest_path);
        let target_dir = rootfs_dir.join(rel_guest);

        if let Some(parent) = target_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        if target_dir.is_symlink() || target_dir.is_file() {
            let _ = fs::remove_file(&target_dir);
        } else if target_dir.is_dir() {
            let _ = fs::remove_dir_all(&target_dir);
        }

        #[cfg(unix)]
        match std::os::unix::fs::symlink(&spec.host_path, &target_dir) {
            Ok(()) => {}
            Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&target_dir);
                let _ = fs::remove_dir_all(&target_dir);
                std::os::unix::fs::symlink(&spec.host_path, &target_dir)?;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
