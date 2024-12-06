#[cfg(target_os = "freebsd")]
pub(crate) mod freebsd;
#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[cfg(target_os = "freebsd")]
pub use freebsd::*;
#[cfg(target_os = "linux")]
pub use linux::*;
