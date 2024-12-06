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

/// # Errors
#[allow(clippy::type_complexity)]
pub fn get_mnt_info() -> Result<Vec<(String, String, String)>, Box<dyn std::error::Error>> {
    Ok(vec![])
}
