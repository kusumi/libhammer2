pub mod chain;
mod extra;
pub mod fs;
pub mod hammer2;
pub mod inode;
pub mod ioctl;
pub mod lz4;
pub mod ondisk;
mod option;
pub mod sha;
pub mod subs;
pub mod volume;
mod xop;
pub mod xxhash;
pub mod zlib;

use std::fmt::Display;

pub const VERSION: [i32; 3] = [0, 4, 1];

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Error(std::io::Error),
    Errno(nix::errno::Errno),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Error(e) => write!(f, "{e}"),
            Self::Errno(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Error(e)
    }
}

impl From<nix::errno::Errno> for Error {
    fn from(e: nix::errno::Errno) -> Self {
        Self::Errno(e)
    }
}

/// # Errors
pub fn mount(spec: &str, args: &[&str]) -> Result<hammer2::Hammer2> {
    hammer2::Hammer2::mount(spec, args)
}

#[must_use]
pub fn is_os_supported() -> bool {
    libfs::os::is_linux() || libfs::os::is_freebsd()
}
