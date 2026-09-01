use std::ffi::CString;
use std::fs::File;
use std::io::{self, Cursor, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub enum ExecutableHandle {
    Memfd {
        _fd: OwnedFd,
        path: PathBuf,
    },
    TempFile {
        _temp_dir: tempfile::TempDir,
        path: PathBuf,
    },
}

impl ExecutableHandle {
    pub fn path(&self) -> &std::path::Path {
        match self {
            ExecutableHandle::Memfd { path, .. } => path,
            ExecutableHandle::TempFile { path, .. } => path,
        }
    }
}

pub fn create_executable_from_zstd(name: &str, zstd_bytes: &[u8]) -> io::Result<ExecutableHandle> {
    let mut decompressed = Vec::new();
    let mut decoder = zstd::stream::read::Decoder::new(Cursor::new(zstd_bytes))?;
    decoder.read_to_end(&mut decompressed)?;

    let c_name = CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let raw_fd = unsafe { libc::memfd_create(c_name.as_ptr(), 0) };

    if raw_fd >= 0 {
        let owned_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        let mut file = File::from(owned_fd);
        file.write_all(&decompressed)?;
        file.flush()?;

        let proc_path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
        let owned_fd: OwnedFd = file.into();

        return Ok(ExecutableHandle::Memfd {
            _fd: owned_fd,
            path: proc_path,
        });
    }

    let temp_dir = tempfile::Builder::new()
        .prefix("qemulbench-bin-")
        .tempdir_in("/dev/shm")
        .or_else(|_| tempfile::Builder::new().prefix("qemulbench-bin-").tempdir())?;

    let file_path = temp_dir.path().join(name);
    let mut file = File::create(&file_path)?;
    file.write_all(&decompressed)?;
    file.flush()?;

    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&file_path, perms)?;

    Ok(ExecutableHandle::TempFile {
        _temp_dir: temp_dir,
        path: file_path,
    })
}

pub fn write_bytes_to_temp_file(name: &str, zstd_bytes: &[u8]) -> io::Result<(tempfile::TempDir, PathBuf)> {
    let mut decompressed = Vec::new();
    let mut decoder = zstd::stream::read::Decoder::new(Cursor::new(zstd_bytes))?;
    decoder.read_to_end(&mut decompressed)?;

    let temp_dir = tempfile::Builder::new()
        .prefix("qemulbench-data-")
        .tempdir_in("/dev/shm")
        .or_else(|_| tempfile::Builder::new().prefix("qemulbench-data-").tempdir())?;

    let file_path = temp_dir.path().join(name);
    let mut file = File::create(&file_path)?;
    file.write_all(&decompressed)?;
    file.flush()?;

    Ok((temp_dir, file_path))
}

pub fn write_raw_bytes_to_temp_file(name: &str, raw_bytes: &[u8]) -> io::Result<(tempfile::TempDir, PathBuf)> {
    let temp_dir = tempfile::Builder::new()
        .prefix("qemulbench-data-")
        .tempdir_in("/dev/shm")
        .or_else(|_| tempfile::Builder::new().prefix("qemulbench-data-").tempdir())?;

    let file_path = temp_dir.path().join(name);
    let mut file = File::create(&file_path)?;
    file.write_all(raw_bytes)?;
    file.flush()?;

    Ok((temp_dir, file_path))
}

pub fn extract_tar_zstd_to_temp_dir(name: &str, zstd_bytes: &[u8]) -> io::Result<tempfile::TempDir> {
    let decoder = zstd::stream::read::Decoder::new(Cursor::new(zstd_bytes))?;
    let mut archive = tar::Archive::new(decoder);

    let temp_dir = tempfile::Builder::new()
        .prefix(&format!("qemulbench-{}-", name))
        .tempdir_in("/dev/shm")
        .or_else(|_| tempfile::Builder::new().prefix(&format!("qemulbench-{}-", name)).tempdir())?;

    archive.unpack(temp_dir.path())?;
    Ok(temp_dir)
}
