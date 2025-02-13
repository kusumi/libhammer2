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
pub unsafe extern "C" fn chmod(path: *const libc::c_char, mode: u32) -> libc::c_int {
    libc::chmod(path, mode as u16)
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn chflags(path: *const libc::c_char, flags: libc::c_ulong) -> libc::c_int {
    libc::chflags(path, flags)
}

/// # Safety
#[must_use]
pub unsafe extern "C" fn lchflags(path: *const libc::c_char, flags: libc::c_ulong) -> libc::c_int {
    libc::lchflags(path, flags)
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
    libc::stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_atime: 0,
        st_atime_nsec: 0,
        st_mtime: 0,
        st_mtime_nsec: 0,
        st_ctime: 0,
        st_ctime_nsec: 0,
        st_size: 0,
        st_blocks: 0,
        st_blksize: 0,
        st_flags: 0,
        st_gen: 0,
        st_lspare: 0,
        st_birthtime: 0,
        st_birthtime_nsec: 0,
    }
}

pub trait StatExt {
    fn get_flags(&self) -> u32;
}

impl StatExt for libc::stat {
    fn get_flags(&self) -> libc::fflags_t {
        self.st_flags
    }
}

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
    let mut mntbufp = std::ptr::null_mut();
    let n = unsafe { getmntinfo(&mut mntbufp, libc::MNT_NOWAIT) };
    if n <= 0 {
        return Ok(vec![]);
    }
    let mut v = vec![];
    for m in unsafe { std::slice::from_raw_parts(mntbufp, n as usize) } {
        let fstypename = crate::util::bin_to_string(unsafe {
            std::slice::from_raw_parts(m.f_fstypename.as_ptr().cast::<u8>(), 16)
        })?;
        let mntonname = crate::util::bin_to_string(unsafe {
            std::slice::from_raw_parts(m.f_mntonname.as_ptr().cast::<u8>(), 1024)
        })?;
        let mntfromname = crate::util::bin_to_string(unsafe {
            std::slice::from_raw_parts(m.f_mntfromname.as_ptr().cast::<u8>(), 1024)
        })?;
        v.push((fstypename, mntonname, mntfromname));
    }
    Ok(v)
}

// taken from flags2opts() in sbin/mount/mount.c
#[must_use]
pub fn get_mount_flag(s: &str) -> Option<nix::mount::MntFlags> {
    Some(match s {
        "acls" => nix::mount::MntFlags::MNT_ACLS,
        "async" => nix::mount::MntFlags::MNT_ASYNC,
        //"emptydir" => nix::mount::MntFlags::MNT_EMPTYDIR,
        "multilabel" => nix::mount::MntFlags::MNT_MULTILABEL,
        "nfsv4acls" => nix::mount::MntFlags::MNT_NFS4ACLS,
        "noatime" => nix::mount::MntFlags::MNT_NOATIME,
        "noclusterr" => nix::mount::MntFlags::MNT_NOCLUSTERR,
        "noclusterw" => nix::mount::MntFlags::MNT_NOCLUSTERW,
        //"nocover" => nix::mount::MntFlags::MNT_NOCOVER,
        "noexec" => nix::mount::MntFlags::MNT_NOEXEC,
        "nosuid" => nix::mount::MntFlags::MNT_NOSUID,
        "nosymfollow" => nix::mount::MntFlags::MNT_NOSYMFOLLOW,
        "ro" => nix::mount::MntFlags::MNT_RDONLY,
        "suiddir" => nix::mount::MntFlags::MNT_SUIDDIR,
        "sync" => nix::mount::MntFlags::MNT_SYNCHRONOUS,
        "union" => nix::mount::MntFlags::MNT_UNION,
        //"untrusted" => nix::mount::MntFlags::MNT_UNTRUSTED,
        "update" => nix::mount::MntFlags::MNT_UPDATE,
        _ => return None,
    })
}
