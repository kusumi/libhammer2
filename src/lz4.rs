/// # Errors
pub fn compress(buf: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut dst = [0; crate::subs::DEBUFSIZE / 2];
    let res = unsafe {
        lz4::liblz4::LZ4_compress_default(
            std::ptr::from_ref::<[u8]>(buf).cast::<i8>(),
            std::ptr::from_mut::<[u8]>(&mut dst[4..]).cast::<i8>(),
            buf.len().try_into()?,
            (dst.len() - 8).try_into()?,
        )
    };
    if res >= 0 {
        dst[..4].copy_from_slice(&res.to_le_bytes());
        Ok(dst[..(4 + res).try_into()?].to_vec())
    } else {
        Err(Box::new(nix::errno::Errno::EINVAL))
    }
}

/// # Errors
/// # Panics
pub fn decompress(buf: &[u8], max_size: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    assert!(max_size <= crate::subs::DEBUFSIZE);
    let mut dst = [0; crate::subs::DEBUFSIZE + 128];
    let cinsize = i32::from_le_bytes(buf[..4].try_into()?);
    assert!(cinsize <= buf.len().try_into()?);
    let res = unsafe {
        lz4::liblz4::LZ4_decompress_safe(
            std::ptr::from_ref::<[u8]>(&buf[4..]).cast::<i8>(),
            std::ptr::from_mut::<[u8]>(&mut dst).cast::<i8>(),
            cinsize,
            max_size.try_into()?,
        )
    };
    if res >= 0 {
        Ok(dst[..res.try_into()?].to_vec())
    } else {
        Err(Box::new(nix::errno::Errno::EINVAL))
    }
}

#[cfg(test)]
mod tests {
    const INPUT: [&[u8]; 4] = [
        b"x",
        b"HAMMER2",
        b"hammer2",
        &[0x41; crate::subs::DEBUFSIZE],
    ];

    #[test]
    fn test_compress_decompress() {
        for &b in &INPUT {
            let c = match super::compress(b) {
                Ok(v) => v,
                Err(e) => panic!("{e}:{b:?}"),
            };
            let d = match super::decompress(&c, crate::subs::DEBUFSIZE) {
                Ok(v) => v,
                Err(e) => panic!("{e}:{b:?}"),
            };
            assert_eq!(d, b, "{b:?}");
        }
    }
}
