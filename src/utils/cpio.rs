use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

const S_IFMT: u32 = 0o170000;
const S_IFDIR: u32 = 0o040000;
const S_IFLNK: u32 = 0o120000;
const S_IFREG: u32 = 0o100000;

const ZERO_PAD: [u8; 3] = [0u8; 3];

fn sanitize_path(target_dir: &Path, rel_path: &str) -> io::Result<PathBuf> {
    let clean = rel_path.trim_start_matches("./").trim_start_matches('/');
    if clean.is_empty() || clean == "." {
        return Ok(target_dir.to_path_buf());
    }

    for component in Path::new(clean).components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid path component in CPIO entry: {}", rel_path),
                ));
            }
        }
    }

    let out_path = target_dir.join(clean);
    if !out_path.starts_with(target_dir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Path traversal outside target directory detected",
        ));
    }

    Ok(out_path)
}

pub fn extract_cpio<R: Read>(mut reader: R, target_dir: &Path) -> io::Result<()> {
    let mut header_buf = [0u8; 110];

    loop {
        if let Err(e) = reader.read_exact(&mut header_buf) {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                break;
            }
            return Err(e);
        }

        let magic = std::str::from_utf8(&header_buf[0..6])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if magic != "070701" && magic != "070702" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid cpio magic: {}", magic),
            ));
        }

        let mode_str = std::str::from_utf8(&header_buf[14..22])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mode = u32::from_str_radix(mode_str, 16)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let filesize_str = std::str::from_utf8(&header_buf[54..62])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let filesize = usize::from_str_radix(filesize_str, 16)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let namesize_str = std::str::from_utf8(&header_buf[94..102])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let namesize = usize::from_str_radix(namesize_str, 16)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut name_buf = vec![0u8; namesize];
        reader.read_exact(&mut name_buf)?;

        let header_name_pad = (4 - ((110 + namesize) % 4)) % 4;
        if header_name_pad > 0 {
            let mut pad = [0u8; 3];
            reader.read_exact(&mut pad[..header_name_pad])?;
        }

        let name_trimmed = if name_buf.last() == Some(&0) {
            &name_buf[..name_buf.len() - 1]
        } else {
            &name_buf[..]
        };

        let file_name = std::str::from_utf8(name_trimmed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if file_name == "TRAILER!!!" {
            break;
        }

        let out_path = sanitize_path(target_dir, file_name)?;
        if out_path == target_dir {
            let file_pad = (4 - (filesize % 4)) % 4;
            if file_pad > 0 {
                let mut pad = [0u8; 3];
                reader.read_exact(&mut pad[..file_pad])?;
            }
            continue;
        }

        let file_type = mode & S_IFMT;

        if file_type == S_IFDIR {
            if out_path.is_symlink() {
                let _ = fs::remove_file(&out_path);
            }
            fs::create_dir_all(&out_path)?;
            let mut perms = fs::metadata(&out_path)?.permissions();
            perms.set_mode((mode & 0o7777) as u32);
            fs::set_permissions(&out_path, perms)?;
        } else if file_type == S_IFLNK {
            let mut link_data = vec![0u8; filesize];
            reader.read_exact(&mut link_data)?;
            let link_target = std::str::from_utf8(&link_data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if out_path.is_symlink() || out_path.exists() {
                let _ = fs::remove_file(&out_path);
            }
            symlink(link_target, &out_path)?;
        } else if file_type == S_IFREG {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if out_path.is_symlink() {
                let _ = fs::remove_file(&out_path);
            }
            let mut out_file = File::create(&out_path)?;
            let mut file_data = vec![0u8; filesize];
            reader.read_exact(&mut file_data)?;
            out_file.write_all(&file_data)?;
            out_file.flush()?;

            let mut perms = out_file.metadata()?.permissions();
            perms.set_mode((mode & 0o7777) as u32);
            fs::set_permissions(&out_path, perms)?;
        } else {
            let mut skip_data = vec![0u8; filesize];
            reader.read_exact(&mut skip_data)?;
        }

        let file_pad = (4 - (filesize % 4)) % 4;
        if file_pad > 0 {
            let mut pad = [0u8; 3];
            reader.read_exact(&mut pad[..file_pad])?;
        }
    }

    Ok(())
}

pub fn extract_rootfs_zstd(zstd_bytes: &[u8]) -> io::Result<tempfile::TempDir> {
    let mut decompressed = Vec::new();
    let mut decoder = zstd::stream::read::Decoder::new(Cursor::new(zstd_bytes))?;
    decoder.read_to_end(&mut decompressed)?;

    let temp_dir = tempfile::Builder::new()
        .prefix("qemulbench-rootfs-")
        .tempdir_in("/dev/shm")
        .or_else(|_| tempfile::Builder::new().prefix("qemulbench-rootfs-").tempdir())?;

    extract_cpio(Cursor::new(decompressed), temp_dir.path())?;
    Ok(temp_dir)
}

pub fn find_cpio_trailer_offset(cpio: &[u8]) -> Option<usize> {
    let mut offset = 0;
    while offset + 110 <= cpio.len() {
        let magic = &cpio[offset..offset + 6];
        if magic != b"070701" && magic != b"070702" {
            break;
        }

        let filesize_str = std::str::from_utf8(&cpio[offset + 54..offset + 62]).ok()?;
        let filesize = usize::from_str_radix(filesize_str, 16).ok()?;

        let namesize_str = std::str::from_utf8(&cpio[offset + 94..offset + 102]).ok()?;
        let namesize = usize::from_str_radix(namesize_str, 16).ok()?;

        let name_start = offset + 110;
        let name_end = name_start + namesize;
        if name_end > cpio.len() {
            break;
        }

        let raw_name = &cpio[name_start..name_end];
        let name_trimmed = if raw_name.last() == Some(&0) {
            &raw_name[..raw_name.len() - 1]
        } else {
            raw_name
        };

        if name_trimmed == b"TRAILER!!!" {
            return Some(offset);
        }

        let header_name_pad = (4 - ((110 + namesize) % 4)) % 4;
        let file_pad = (4 - (filesize % 4)) % 4;
        offset = name_end + header_name_pad + filesize + file_pad;
    }
    None
}

fn write_cpio_entry<W: Write>(
    writer: &mut W,
    path_name: &str,
    mode: u32,
    content: &[u8],
) -> io::Result<()> {
    let name_bytes = path_name.as_bytes();
    let name_len = name_bytes.len() + 1;
    let content_len = content.len();

    let header = format!(
        "070701{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
        0,
        mode,
        0,
        0,
        1,
        0,
        content_len,
        0,
        0,
        0,
        0,
        name_len,
        0,
    );

    writer.write_all(header.as_bytes())?;
    writer.write_all(name_bytes)?;
    writer.write_all(&[0])?;

    let name_pad = (4 - ((110 + name_len) % 4)) % 4;
    if name_pad > 0 {
        writer.write_all(&ZERO_PAD[..name_pad])?;
    }

    if content_len > 0 {
        writer.write_all(content)?;
        let content_pad = (4 - (content_len % 4)) % 4;
        if content_pad > 0 {
            writer.write_all(&ZERO_PAD[..content_pad])?;
        }
    }

    Ok(())
}

fn add_fs_item_to_cpio<W: Write>(
    writer: &mut W,
    host_item: &Path,
    guest_dest: &str,
    created_dirs: &mut HashSet<String>,
) -> io::Result<()> {
    let meta = fs::symlink_metadata(host_item)?;
    let clean_guest = guest_dest.trim_start_matches('/');

    if meta.is_dir() {
        if created_dirs.insert(clean_guest.to_string()) {
            write_cpio_entry(writer, clean_guest, S_IFDIR | 0o755, &[])?;
        }
        for entry_res in fs::read_dir(host_item)? {
            let entry = entry_res?;
            let child_name = entry.file_name();
            let child_guest = format!("{}/{}", clean_guest, child_name.to_string_lossy());
            add_fs_item_to_cpio(writer, &entry.path(), &child_guest, created_dirs)?;
        }
    } else if meta.is_file() {
        let mut data = Vec::new();
        File::open(host_item)?.read_to_end(&mut data)?;
        let mode = S_IFREG | (meta.permissions().mode() & 0o7777);
        write_cpio_entry(writer, clean_guest, mode, &data)?;
    } else if meta.file_type().is_symlink() {
        let link_target = fs::read_link(host_item)?;
        let target_bytes = link_target.to_string_lossy().as_bytes().to_vec();
        write_cpio_entry(writer, clean_guest, S_IFLNK | 0o777, &target_bytes)?;
    }

    Ok(())
}

pub fn create_overlay_cpio(mounts: &[crate::utils::mount::MountSpec]) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut created_dirs = HashSet::new();

    for spec in mounts {
        let is_file = spec.host_path.is_file();
        let guest_str = spec.guest_path.to_string_lossy();
        let clean = guest_str.trim_start_matches('/');
        let parts: Vec<&str> = clean.split('/').filter(|p| !p.is_empty()).collect();

        let dir_parts_len = if is_file && !parts.is_empty() {
            parts.len() - 1
        } else {
            parts.len()
        };

        let mut acc = String::new();
        for &part in parts.iter().take(dir_parts_len) {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(part);
            if created_dirs.insert(acc.clone()) {
                write_cpio_entry(&mut buffer, &acc, S_IFDIR | 0o755, &[])?;
            }
        }

        add_fs_item_to_cpio(&mut buffer, &spec.host_path, clean, &mut created_dirs)?;
    }

    write_cpio_entry(&mut buffer, "TRAILER!!!", 0, &[])?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::utils::mount::MountSpec;

    #[test]
    fn test_overlay_cpio() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let spec = MountSpec {
            host_path: tmp.path().to_path_buf(),
            guest_path: PathBuf::from("/mnt/custom"),
        };

        let cpio_bytes = create_overlay_cpio(&[spec]).unwrap();
        let trailer_pos = find_cpio_trailer_offset(&cpio_bytes);
        assert!(trailer_pos.is_some());

        let target_dir = tempfile::tempdir().unwrap();
        extract_cpio(Cursor::new(cpio_bytes), target_dir.path()).unwrap();

        let extracted_file = target_dir.path().join("mnt/custom/test.txt");
        assert!(extracted_file.exists());
        assert_eq!(fs::read_to_string(extracted_file).unwrap(), "hello world");
    }

    #[test]
    fn test_overlay_single_file_cpio() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("script.sh");
        fs::write(&file_path, "#!/bin/sh\necho ok").unwrap();

        let spec = MountSpec {
            host_path: file_path,
            guest_path: PathBuf::from("/bin/script.sh"),
        };

        let cpio_bytes = create_overlay_cpio(&[spec]).unwrap();
        let target_dir = tempfile::tempdir().unwrap();
        extract_cpio(Cursor::new(cpio_bytes), target_dir.path()).unwrap();

        let extracted_file = target_dir.path().join("bin/script.sh");
        assert!(extracted_file.is_file());
    }
}
