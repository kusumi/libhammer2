use std::io::Seek;

#[allow(unused_macros)]
#[macro_export]
macro_rules! new_cstring {
    ($s:expr) => {
        std::ffi::CString::new($s)
    };
}
pub use new_cstring;

/// # Errors
pub fn bin_to_string(b: &[u8]) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(
        match b.iter().position(|&x| x == 0) {
            Some(v) => &b[..v],
            None => b,
        }
        .to_vec(),
    )
}

/// # Errors
pub fn get_current_time() -> Result<u64, std::time::SystemTimeError> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

/// # Errors
pub fn open(path: &str, readonly: bool) -> std::io::Result<std::fs::File> {
    if readonly {
        std::fs::File::open(path)
    } else {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
    }
}

/// # Errors
pub fn seek_set(fp: &mut std::fs::File, offset: u64) -> std::io::Result<u64> {
    fp.seek(std::io::SeekFrom::Start(offset))
}

/// # Errors
pub fn seek_end(fp: &mut std::fs::File, offset: i64) -> std::io::Result<u64> {
    fp.seek(std::io::SeekFrom::End(offset))
}

pub(crate) fn split_path(path: &str) -> Vec<&str> {
    let mut v = vec![];
    for x in &path.trim_matches('/').split('/').collect::<Vec<&str>>() {
        // multiple /'s between components generates ""
        if !x.is_empty() && *x != "." {
            v.push(*x);
        }
    }
    v
}

/// # Panics
#[must_use]
pub fn align_head_to<T>(buf: &[u8]) -> &T {
    let (prefix, body, _) = unsafe { buf.align_to::<T>() };
    assert!(prefix.is_empty(), "{:?} {}", prefix, prefix.len());
    &body[0]
}

/// # Panics
#[must_use]
pub fn align_to<T>(buf: &[u8]) -> &T {
    let (prefix, body, suffix) = unsafe { buf.align_to::<T>() };
    assert!(prefix.is_empty(), "{:?} {}", prefix, prefix.len());
    assert!(suffix.is_empty(), "{:?} {}", suffix, suffix.len());
    &body[0]
}

/// # Panics
pub fn align_head_to_mut<T>(buf: &mut [u8]) -> &mut T {
    let (prefix, body, _) = unsafe { buf.align_to_mut::<T>() };
    assert!(prefix.is_empty(), "{:?} {}", prefix, prefix.len());
    &mut body[0]
}

/// # Panics
pub fn align_to_mut<T>(buf: &mut [u8]) -> &mut T {
    let (prefix, body, suffix) = unsafe { buf.align_to_mut::<T>() };
    assert!(prefix.is_empty(), "{:?} {}", prefix, prefix.len());
    assert!(suffix.is_empty(), "{:?} {}", suffix, suffix.len());
    &mut body[0]
}

/// # Safety
pub fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe {
        ::core::slice::from_raw_parts(
            std::ptr::from_ref::<T>(p).cast::<u8>(),
            ::core::mem::size_of::<T>(),
        )
    }
}

#[must_use]
pub fn get_os_name() -> &'static str {
    std::env::consts::OS
}

#[must_use]
pub fn is_os_supported() -> bool {
    is_linux() || is_freebsd()
}

#[must_use]
pub fn is_linux() -> bool {
    get_os_name() == "linux"
}

#[must_use]
pub fn is_freebsd() -> bool {
    get_os_name() == "freebsd"
}

#[must_use]
pub fn is_solaris() -> bool {
    get_os_name() == "solaris"
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bin_to_string() {
        assert_eq!(
            super::bin_to_string(&[104, 97, 109, 109, 101, 114, 50]),
            Ok("hammer2".to_string())
        );
        assert_eq!(
            super::bin_to_string(&[104, 97, 109, 109, 101, 114, 50, 0]),
            Ok("hammer2".to_string())
        );
        assert_eq!(
            super::bin_to_string(&[104, 97, 109, 109, 101, 114, 50, 0, 0]),
            Ok("hammer2".to_string())
        );

        assert_eq!(super::bin_to_string(&[]), Ok(String::new()));
        assert_eq!(super::bin_to_string(&[0]), Ok(String::new()));
        assert_eq!(super::bin_to_string(&[0, 0]), Ok(String::new()));
        assert_eq!(
            super::bin_to_string(&[0, 0, 104, 97, 109, 109, 101, 114, 50]),
            Ok(String::new())
        );
    }

    #[test]
    fn test_get_current_time() {
        let t1 = match super::get_current_time() {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        let t2 = match super::get_current_time() {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        assert_ne!(t1, 0);
        assert_ne!(t2, 0);
        assert!(t2 >= t1);
    }

    #[test]
    fn test_split_path() {
        assert!(super::split_path("").is_empty());

        assert!(super::split_path("/").is_empty());
        assert!(super::split_path("/.").is_empty());

        assert!(super::split_path("//").is_empty());
        assert!(super::split_path("//.").is_empty());

        assert!(super::split_path(".").is_empty());
        assert!(super::split_path("./.").is_empty());

        assert_eq!(super::split_path(" "), [" "]);
        assert_eq!(super::split_path(".."), [".."]);
        assert_eq!(super::split_path("cnp"), ["cnp"]);

        assert_eq!(super::split_path("/cnp"), ["cnp"]);
        assert_eq!(super::split_path("//cnp"), ["cnp"]);
        assert_eq!(super::split_path("./cnp"), ["cnp"]);

        assert_eq!(super::split_path("cnp/"), ["cnp"]);
        assert_eq!(super::split_path("cnp//"), ["cnp"]);
        assert_eq!(super::split_path("cnp/."), ["cnp"]);

        assert_eq!(super::split_path("/cnp/"), ["cnp"]);
        assert_eq!(super::split_path("//cnp//"), ["cnp"]);
        assert_eq!(super::split_path("./cnp/."), ["cnp"]);

        assert_eq!(super::split_path("/path/to/cnp"), ["path", "to", "cnp"]);
        assert_eq!(
            super::split_path("///path///to///cnp///"),
            ["path", "to", "cnp"]
        );
        assert_eq!(
            super::split_path("./path/./to/./cnp/."),
            ["path", "to", "cnp"]
        );
    }
}
