pub mod extra;
pub mod fs;
pub mod ioctl;
pub mod lz4;
pub mod ondisk;
pub mod os;
pub mod sha;
pub mod subs;
pub mod util;
pub mod volume;
pub mod xxhash;
pub mod zlib;

use std::fmt::Display;

pub const VERSION: [i32; 3] = [0, 3, 0];

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
