use byteorder::ByteOrder;
use std::os::unix::fs::FileTypeExt;

pub(crate) const DEBUFSIZE: usize = crate::fs::HAMMER2_PBUFSIZE as usize;
pub(crate) const DEV_BSIZE: u64 = 512;

pub const K: usize = 1024;
pub const M: usize = K * 1024;
pub const G: usize = M * 1024;
pub const T: usize = G * 1024;

pub const K_U64: u64 = K as u64;
pub const M_U64: u64 = M as u64;
pub const G_U64: u64 = G as u64;
pub const T_U64: u64 = T as u64;

pub const K_F64: f64 = K as f64;
pub const M_F64: f64 = M as f64;
pub const G_F64: f64 = G as f64;
pub const T_F64: f64 = T as f64;

const FORCE_STD_EPOCH: &str = "HAMMER2_FORCE_STD_EPOCH";

fn get_local_time_delta(t: u64) -> Result<i64, time::error::IndeterminateOffset> {
    let mut d = i64::from(time::UtcOffset::current_local_offset()?.whole_seconds());
    if t == 0 && std::env::var(FORCE_STD_EPOCH).is_ok() {
        d -= 3600;
    }
    Ok(d)
}

/// # Panics
#[must_use]
pub fn get_local_time_string(t: u64) -> String {
    get_time_string_impl(t, get_local_time_delta(t).unwrap()).unwrap()
}

/// # Panics
#[must_use]
pub fn get_time_string(t: u64) -> String {
    get_time_string_impl(t, 0).unwrap()
}

fn get_time_string_impl(t: u64, d: i64) -> Result<String, Box<dyn std::error::Error>> {
    let t = i64::try_from(t / 1_000_000)? + d;
    let t = if t < 0 {
        std::time::SystemTime::UNIX_EPOCH - std::time::Duration::from_secs((-t).try_into()?)
    } else {
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t.try_into()?)
    };
    let fmt = time::format_description::parse(
        "[day]-[month repr:short]-[year] [hour]:[minute]:[second]",
    )?;
    Ok(time::OffsetDateTime::from(t).format(&fmt)?)
}

#[must_use]
pub fn get_inode_type_string(typ: u8) -> &'static str {
    match typ {
        crate::fs::HAMMER2_OBJTYPE_UNKNOWN => "UNKNOWN",
        crate::fs::HAMMER2_OBJTYPE_DIRECTORY => "DIR",
        crate::fs::HAMMER2_OBJTYPE_REGFILE => "FILE",
        crate::fs::HAMMER2_OBJTYPE_FIFO => "FIFO",
        crate::fs::HAMMER2_OBJTYPE_CDEV => "CDEV",
        crate::fs::HAMMER2_OBJTYPE_BDEV => "BDEV",
        crate::fs::HAMMER2_OBJTYPE_SOFTLINK => "SOFTLINK",
        crate::fs::HAMMER2_OBJTYPE_SOCKET => "SOCKET",
        crate::fs::HAMMER2_OBJTYPE_WHITEOUT => "WHITEOUT",
        _ => "ILLEGAL",
    }
}

#[must_use]
pub fn get_pfs_type_string(typ: u8) -> &'static str {
    match typ {
        crate::fs::HAMMER2_PFSTYPE_NONE => "NONE",
        crate::fs::HAMMER2_PFSTYPE_SUPROOT => "SUPROOT",
        crate::fs::HAMMER2_PFSTYPE_DUMMY => "DUMMY",
        crate::fs::HAMMER2_PFSTYPE_CACHE => "CACHE",
        crate::fs::HAMMER2_PFSTYPE_SLAVE => "SLAVE",
        crate::fs::HAMMER2_PFSTYPE_SOFT_SLAVE => "SOFT_SLAVE",
        crate::fs::HAMMER2_PFSTYPE_SOFT_MASTER => "SOFT_MASTER",
        crate::fs::HAMMER2_PFSTYPE_MASTER => "MASTER",
        _ => "ILLEGAL",
    }
}

#[must_use]
pub fn get_pfs_subtype_string(typ: u8) -> &'static str {
    match typ {
        crate::fs::HAMMER2_PFSSUBTYPE_NONE => "NONE",
        crate::fs::HAMMER2_PFSSUBTYPE_SNAPSHOT => "SNAPSHOT",
        crate::fs::HAMMER2_PFSSUBTYPE_AUTOSNAP => "AUTOSNAP",
        _ => "ILLEGAL",
    }
}

#[must_use]
pub fn get_blockref_type_string(typ: u8) -> &'static str {
    match typ {
        crate::fs::HAMMER2_BREF_TYPE_EMPTY => "empty",
        crate::fs::HAMMER2_BREF_TYPE_INODE => "inode",
        crate::fs::HAMMER2_BREF_TYPE_INDIRECT => "indirect",
        crate::fs::HAMMER2_BREF_TYPE_DATA => "data",
        crate::fs::HAMMER2_BREF_TYPE_DIRENT => "dirent",
        crate::fs::HAMMER2_BREF_TYPE_FREEMAP_NODE => "freemap_node",
        crate::fs::HAMMER2_BREF_TYPE_FREEMAP_LEAF => "freemap_leaf",
        crate::fs::HAMMER2_BREF_TYPE_INVALID => "invalid",
        crate::fs::HAMMER2_BREF_TYPE_FREEMAP => "freemap",
        crate::fs::HAMMER2_BREF_TYPE_VOLUME => "volume",
        _ => "unknown",
    }
}

pub const HAMMER2_CHECK_STRINGS: [&str; 6] =
    ["none", "disabled", "crc32", "xxhash64", "sha192", "freemap"];
pub const HAMMER2_COMP_STRINGS: [&str; 4] = ["none", "autozero", "lz4", "zlib"];

// Note: Check algorithms normally do not encode any level.
#[must_use]
pub fn get_check_mode_string(x: u8) -> String {
    let check = usize::from(crate::fs::dec_algo(x));
    let level = crate::fs::dec_level(x);
    if level != 0 {
        if check < HAMMER2_CHECK_STRINGS.len() {
            format!("{}:{level}", HAMMER2_CHECK_STRINGS[check])
        } else {
            format!("unknown({check}):{level}")
        }
    } else if true {
        if check < HAMMER2_CHECK_STRINGS.len() {
            HAMMER2_CHECK_STRINGS[check].to_string()
        } else {
            format!("unknown({check})")
        }
    } else {
        unreachable!();
    }
}

#[must_use]
pub fn get_comp_mode_string(x: u8) -> String {
    let comp = usize::from(crate::fs::dec_algo(x));
    let level = crate::fs::dec_level(x);
    if level != 0 {
        if comp < HAMMER2_COMP_STRINGS.len() {
            format!("{}:{level}", HAMMER2_COMP_STRINGS[comp])
        } else {
            format!("unknown({comp}):{level}")
        }
    } else if true {
        if comp < HAMMER2_COMP_STRINGS.len() {
            format!("{}:default", HAMMER2_COMP_STRINGS[comp])
        } else {
            format!("unknown({comp}):default")
        }
    } else {
        unreachable!();
    }
}

#[must_use]
pub fn get_size_string(size: u64) -> String {
    if size < K_U64 / 2 {
        format!("{:6.2}B", size as f64)
    } else if size < M_U64 / 2 {
        format!("{:6.2}KB", size as f64 / K_F64)
    } else if size < G_U64 / 2 {
        format!("{:6.2}MB", size as f64 / M_F64)
    } else if size < T_U64 / 2 {
        format!("{:6.2}GB", size as f64 / G_F64)
    } else {
        format!("{:6.2}TB", size as f64 / T_F64)
    }
}

#[must_use]
pub fn get_count_string(size: u64) -> String {
    if size < M_U64 / 2 {
        format!("{size}")
    } else if size < G_U64 / 2 {
        format!("{:6.2}M", size as f64 / M_F64)
    } else if size < T_U64 / 2 {
        format!("{:6.2}G", size as f64 / G_F64)
    } else {
        format!("{:6.2}T", size as f64 / T_F64)
    }
}

/// # Errors
pub fn get_volume_size_from_path(f: &str) -> crate::Result<u64> {
    get_volume_size(&mut std::fs::File::open(f)?)
}

/// # Errors
pub fn get_volume_size(fp: &mut std::fs::File) -> crate::Result<u64> {
    let t = fp.metadata()?.file_type();
    if !t.is_block_device() && !t.is_char_device() && !t.is_file() {
        log::error!("{fp:?}: unsupported type {t:?}");
        return Err(nix::errno::Errno::EINVAL.into());
    }

    if crate::util::is_linux() || crate::util::is_freebsd() || crate::util::is_solaris() {
        let size = crate::util::seek_end(fp, 0)?;
        if size == 0 {
            log::error!("{fp:?}: failed to get size");
            return Err(nix::errno::Errno::EINVAL.into());
        }
        crate::util::seek_set(fp, 0)?;
        Ok(size)
    } else {
        // XXX other platforms use ioctl(2)
        log::error!("{} is unsupported", crate::util::get_os_name());
        Err(nix::errno::Errno::EOPNOTSUPP.into())
    }
}

// Borrow HAMMER1's directory hash algorithm #1 with a few modifications.
// The filename is split into fields which are hashed separately and then
// added together.
//
// Differences include: bit 63 must be set to 1 for HAMMER2 (HAMMER1 sets
// it to 0), this is because bit63=0 is used for hidden hardlinked inodes.
// (This means we do not need to do a 0-check/or-with-0x100000000 either).
//
// Also, the iscsi crc code is used instead of the old crc32 code.
#[must_use]
pub fn dirhash(aname: &[u8]) -> u64 {
    // m32
    let mut crcx = 0;
    let mut i = 0;
    let mut j = 0;
    while i < aname.len() {
        let x = aname[i] as char;
        if x == '.' || x == '-' || x == '_' || x == '~' {
            if i != j {
                crcx += icrc32::iscsi_crc32(&aname[j..i]);
            }
            j = i + 1;
        }
        i += 1;
    }
    if i != j {
        crcx += icrc32::iscsi_crc32(&aname[j..i]);
    }

    // The directory hash utilizes the top 32 bits of the 64-bit key.
    // Bit 63 must be set to 1.
    crcx |= 0x8000_0000;
    let mut key = u64::from(crcx) << 32;

    // l16 - crc of entire filename
    // This crc reduces degenerate hash collision conditions.
    let mut crcx = icrc32::iscsi_crc32(aname);
    crcx = crcx ^ (crcx << 16);
    key |= u64::from(crcx) & 0xFFFF_0000;

    // Set bit 15.  This allows readdir to strip bit 63 so a positive
    // 64-bit cookie/offset can always be returned, and still guarantee
    // that the values 0x0000-0x7FFF are available for artificial entries
    // ('.' and '..').
    key | 0x8000
}

/// # Errors
pub fn get_hammer2_mounts() -> Result<Vec<String>, std::string::FromUtf8Error> {
    let mut v = vec![];
    for (fstypename, mntonname, _) in crate::os::get_mnt_info()? {
        if fstypename == "hammer2" {
            v.push(mntonname);
        }
    }
    Ok(v)
}

/// # Errors
pub fn get_uuid_from_str(s: &str) -> Result<uuid::Uuid, uuid::Error> {
    let src = *uuid::Uuid::parse_str(s)?.as_bytes();
    let mut dst = src;
    dst[0] = src[3]; // 4
    dst[1] = src[2];
    dst[2] = src[1];
    dst[3] = src[0];
    dst[4] = src[5]; // 2
    dst[5] = src[4];
    dst[6] = src[7]; // 2
    dst[7] = src[6];
    Ok(uuid::Uuid::from_bytes(dst))
}

#[must_use]
pub fn get_uuid_string(u: &uuid::Uuid) -> String {
    get_uuid_string_from_bytes(u.as_bytes())
}

#[must_use]
pub fn get_uuid_string_from_bytes(b: &[u8]) -> String {
    format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[3], b[2], b[1], b[0], // 4
        b[5], b[4], // 2
        b[7], b[6], // 2
        b[8], b[9],
        b[10], b[11], b[12], b[13], b[14], b[15])
}

pub(crate) fn conv_time_to_timespec(t: u64) -> u64 {
    t / 1_000_000 // sec
}

#[allow(dead_code)]
pub(crate) fn conv_timespec_to_time(t: u64) -> u64 {
    t * 1_000_000 // sec
}

#[allow(dead_code)]
pub(crate) fn conv_uuid_to_unix_xid(u: &uuid::Uuid) -> u32 {
    conv_uuid_to_unix_xid_from_bytes(u.as_bytes())
}

pub(crate) fn conv_uuid_to_unix_xid_from_bytes(b: &[u8]) -> u32 {
    byteorder::NativeEndian::read_u32(&b[12..])
}

#[allow(dead_code)]
pub(crate) fn conv_unix_xid_to_uuid(xid: u32) -> uuid::Uuid {
    uuid::Uuid::from_bytes(conv_unix_xid_to_uuid_bytes(xid))
}

pub(crate) fn conv_unix_xid_to_uuid_bytes(xid: u32) -> [u8; 16] {
    let mut uuid = [0; 16];
    byteorder::NativeEndian::write_u32_into(&[xid], &mut uuid[12..]);
    uuid
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_time_string() {
        assert_eq!(super::get_time_string(0), "01-Jan-1970 00:00:00");
        assert_eq!(super::get_time_string(1_000_000), "01-Jan-1970 00:00:01");
    }

    #[test]
    fn test_get_time_string_impl() {
        assert_eq!(
            match super::get_time_string_impl(0, -25200) {
                Ok(v) => v,
                Err(e) => panic!("{e}"),
            },
            "31-Dec-1969 17:00:00".to_string()
        ); // -7
        assert_eq!(
            match super::get_time_string_impl(0, 32400) {
                Ok(v) => v,
                Err(e) => panic!("{e}"),
            },
            "01-Jan-1970 09:00:00".to_string()
        ); // +9
    }

    #[test]
    fn test_get_check_mode_string() {
        let l1 = crate::fs::enc_level(1);
        let l0 = crate::fs::enc_level(0);
        let def_algo = crate::fs::enc_algo(crate::fs::HAMMER2_CHECK_DEFAULT);
        assert_eq!(super::get_check_mode_string(l1 | def_algo), "xxhash64:1");
        assert_eq!(super::get_check_mode_string(l1 | 0xf), "unknown(15):1");
        assert_eq!(super::get_check_mode_string(l0 | def_algo), "xxhash64");
        assert_eq!(super::get_check_mode_string(l0 | 0xf), "unknown(15)");
    }

    #[test]
    fn test_get_comp_mode_string() {
        let l1 = crate::fs::enc_level(1);
        let l0 = crate::fs::enc_level(0);
        let def_algo = crate::fs::enc_algo(crate::fs::HAMMER2_COMP_DEFAULT);
        assert_eq!(super::get_comp_mode_string(l1 | def_algo), "lz4:1");
        assert_eq!(super::get_comp_mode_string(l1 | 0xf), "unknown(15):1");
        assert_eq!(super::get_comp_mode_string(l0 | def_algo), "lz4:default");
        assert_eq!(super::get_comp_mode_string(l0 | 0xf), "unknown(15):default");
    }

    #[test]
    fn test_get_size_string() {
        let l = [
            (0, "  0.00B"),
            (1, "  1.00B"),
            (512, "  0.50KB"),
            (1024, "  1.00KB"),
            (524_288, "  0.50MB"),
            (1_048_576, "  1.00MB"),
        ];
        for t in &l {
            assert_eq!(super::get_size_string(t.0), t.1, "{}", t.0);
        }
    }

    #[test]
    fn test_dirhash() {
        let l = [
            ("", 0x8000_0000_0000_8000),
            (".", 0x8000_0000_bc10_8000),
            ("-", 0x8000_0000_5cb4_8000),
            ("_", 0x8000_0000_e8ed_8000),
            ("~", 0x8000_0000_37e6_8000),
            ("A", 0xe16d_cdee_2c83_8000),
            ("hammer2", 0x9f2f_13b5_8c9a_8000),
            (
                "This code is derived from software contributed to The DragonFly Project",
                0xf8df_95ed_6d32_8000,
            ),
        ];
        for t in &l {
            assert_eq!(super::dirhash(t.0.as_bytes()), t.1, "{}", t.0);
        }
    }

    #[test]
    fn test_uuid_parse_str() {
        let u = match uuid::Uuid::parse_str(crate::fs::HAMMER2_UUID_STRING) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(u.to_string(), crate::fs::HAMMER2_UUID_STRING);
    }

    #[test]
    fn test_get_uuid_string() {
        let u = match super::get_uuid_from_str(crate::fs::HAMMER2_UUID_STRING) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(super::get_uuid_string(&u), crate::fs::HAMMER2_UUID_STRING);
        assert_eq!(
            super::get_uuid_string_from_bytes(crate::util::any_as_u8_slice(&u)),
            crate::fs::HAMMER2_UUID_STRING
        );
    }

    #[test]
    fn test_conv_uuid_to_unix_xid() {
        let u = match super::get_uuid_from_str(crate::fs::HAMMER2_UUID_STRING) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        let node = &u.as_bytes()[10..];
        assert_eq!(node.len(), 6);
        let xid = (u32::from(node[5]) << 24)
            + (u32::from(node[4]) << 16)
            + (u32::from(node[3]) << 8)
            + u32::from(node[2]);
        assert_eq!(super::conv_uuid_to_unix_xid(&u), xid);
        assert_eq!(super::conv_uuid_to_unix_xid_from_bytes(u.as_bytes()), xid);
        // node: 01 30 1b b8 a9 f5
        let xid = (0xf5_u32 << 24) + (0xa9_u32 << 16) + (0xb8_u32 << 8) + 0x1b_u32;
        assert_eq!(super::conv_uuid_to_unix_xid(&u), xid);
        assert_eq!(super::conv_uuid_to_unix_xid_from_bytes(u.as_bytes()), xid);
    }

    #[test]
    fn test_conv_unix_xid_to_uuid() {
        let xid = 0x1234_5678;
        let b = super::conv_unix_xid_to_uuid_bytes(xid);
        let u = super::conv_unix_xid_to_uuid(xid);
        assert_eq!(b, *u.as_bytes());
        let b = u.as_bytes();
        assert_eq!(b[0], 0);
        assert_eq!(b[1], 0);
        assert_eq!(b[2], 0);
        assert_eq!(b[3], 0);
        assert_eq!(b[4], 0);
        assert_eq!(b[5], 0);
        assert_eq!(b[6], 0);
        assert_eq!(b[7], 0);
        assert_eq!(b[8], 0);
        assert_eq!(b[9], 0);
        assert_eq!(b[10], 0);
        assert_eq!(b[11], 0);
        assert!(b[12] == 0x12 || b[12] == 0x78);
        assert!(b[13] == 0x34 || b[13] == 0x56);
        assert!(b[14] == 0x56 || b[14] == 0x34);
        assert!(b[15] == 0x78 || b[15] == 0x12);
    }
}
