/// # Errors
pub fn compress(buf: &[u8], level: u8) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(miniz_oxide::deflate::compress_to_vec_zlib(buf, level))
}

/// # Errors
pub fn decompress(buf: &[u8], max_size: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(
        buf, max_size,
    )?)
}

#[cfg(test)]
mod tests {
    use crate::subs::DEBUFSIZE;

    const INPUT: [&[u8]; 4] = [b"x", b"HAMMER2", b"hammer2", &[0x41; DEBUFSIZE]];

    #[test]
    fn test_compress_decompress() {
        for &b in &INPUT {
            for level in 0..=9 {
                let c = match super::compress(b, level) {
                    Ok(v) => v,
                    Err(e) => panic!("{e}:{b:?}:{level}"),
                };
                let d = match super::decompress(&c, DEBUFSIZE) {
                    Ok(v) => v,
                    Err(e) => panic!("{e}:{b:?}:{level}"),
                };
                assert_eq!(d, b, "{b:?}:{level}");
            }
        }
    }
}
