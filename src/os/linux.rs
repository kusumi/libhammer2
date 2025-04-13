pub use std::os::linux::fs::MetadataExt;

pub const NAME_MAX: usize = 255;
pub const MAXPATHLEN: usize = 1024;

/// # Safety
pub unsafe extern "C" fn getmntinfo(
    _mntbufp: *mut *mut libc::statfs,
    _flags: libc::c_int,
) -> libc::c_int {
    -1
}

/// # Safety
pub unsafe extern "C" fn sysctlbyname(
    _name: *const libc::c_char,
    _oldp: *mut libc::c_void,
    _oldlenp: *mut libc::size_t,
    _newp: *mut libc::c_void,
    _newlen: libc::size_t,
) -> libc::c_int {
    -1
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn stat(path: *const libc::c_char, buf: *mut libc::stat) -> libc::c_int {
    libc::stat(path, buf)
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn chown(
    path: *const libc::c_char,
    uid: libc::uid_t,
    gid: libc::gid_t,
) -> libc::c_int {
    libc::chown(path, uid, gid)
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn lchown(
    path: *const libc::c_char,
    uid: libc::uid_t,
    gid: libc::gid_t,
) -> libc::c_int {
    libc::lchown(path, uid, gid)
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn chmod(path: *const libc::c_char, mode: libc::mode_t) -> libc::c_int {
    libc::chmod(path, mode)
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn chflags(_path: *const libc::c_char, _flags: libc::c_ulong) -> libc::c_int {
    -1
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn lchflags(
    _path: *const libc::c_char,
    _flags: libc::c_ulong,
) -> libc::c_int {
    -1
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn utimes(
    path: *const libc::c_char,
    times: *const libc::timeval,
) -> libc::c_int {
    libc::utimes(path, times)
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn lutimes(
    path: *const libc::c_char,
    times: *const libc::timeval,
) -> libc::c_int {
    libc::lutimes(path, times)
}

#[must_use]
pub fn new_stat() -> libc::stat {
    // libc::stat contains private fields
    // https://docs.rs/libc/latest/libc/struct.stat.html
    unsafe { std::mem::zeroed() }
}

pub trait StatExt {
    fn get_flags(&self) -> u32;
}

impl StatExt for libc::stat {
    fn get_flags(&self) -> u32 {
        0
    }
}

#[allow(clippy::similar_names)]
#[must_use]
pub fn new_timeval(tv_sec: u64, tv_usec: u64) -> libc::timeval {
    libc::timeval {
        tv_sec: tv_sec as libc::time_t,
        tv_usec: tv_usec as libc::suseconds_t,
    }
}

/// # Errors
#[allow(clippy::type_complexity)]
pub fn get_mnt_info() -> Result<Vec<(String, String, String)>, std::string::FromUtf8Error> {
    Ok(vec![])
}

#[must_use]
pub fn get_mount_flag(_s: &str) -> Option<nix::mount::MntFlags> {
    None
}
