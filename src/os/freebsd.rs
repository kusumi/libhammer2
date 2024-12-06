use crate::util;

pub use std::os::freebsd::fs::MetadataExt;

pub const NAME_MAX: usize = 255;
pub const MAXPATHLEN: usize = 1024;

/// # Safety
pub unsafe extern "C" fn getmntinfo(
    mntbufp: *mut *mut libc::statfs,
    flags: libc::c_int,
) -> libc::c_int {
    libc::getmntinfo(mntbufp, flags)
}

/// # Safety
pub unsafe extern "C" fn sysctlbyname(
    name: *const libc::c_char,
    oldp: *mut libc::c_void,
    oldlenp: *mut libc::size_t,
    newp: *mut libc::c_void,
    newlen: libc::size_t,
) -> libc::c_int {
    libc::sysctlbyname(name, oldp, oldlenp, newp, newlen)
}

/// # Errors
#[allow(clippy::type_complexity)]
pub fn get_mnt_info() -> Result<Vec<(String, String, String)>, Box<dyn std::error::Error>> {
    let mut mntbufp = std::ptr::null_mut();
    let n = unsafe { getmntinfo(&mut mntbufp, libc::MNT_NOWAIT) };
    if n <= 0 {
        return Ok(vec![]);
    }
    let mut v = vec![];
    for m in unsafe { std::slice::from_raw_parts(mntbufp, n.try_into()?) } {
        let fstypename = util::bin_to_string(unsafe {
            std::slice::from_raw_parts(m.f_fstypename.as_ptr().cast::<u8>(), 16)
        })?;
        let mntonname = util::bin_to_string(unsafe {
            std::slice::from_raw_parts(m.f_mntonname.as_ptr().cast::<u8>(), 1024)
        })?;
        let mntfromname = util::bin_to_string(unsafe {
            std::slice::from_raw_parts(m.f_mntfromname.as_ptr().cast::<u8>(), 1024)
        })?;
        v.push((fstypename, mntonname, mntfromname));
    }
    Ok(v)
}
