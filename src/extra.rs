impl crate::fs::Hammer2Blockref {
    #[must_use]
    pub fn embed_as<T>(&self) -> &T {
        crate::util::align_to(&self.embed)
    }

    pub fn embed_as_mut<T>(&mut self) -> &mut T {
        crate::util::align_to_mut(&mut self.embed)
    }

    #[must_use]
    pub fn check_as<T>(&self) -> &T {
        crate::util::align_to(&self.check)
    }

    pub fn check_as_mut<T>(&mut self) -> &mut T {
        crate::util::align_to_mut(&mut self.check)
    }
}

impl crate::fs::Hammer2Blockset {
    #[must_use]
    pub fn as_blockref(&self) -> [&crate::fs::Hammer2Blockref; crate::fs::HAMMER2_SET_COUNT] {
        [
            &self.blockref[0],
            &self.blockref[1],
            &self.blockref[2],
            &self.blockref[3],
        ]
    }
}

impl crate::fs::Hammer2InodeMeta {
    #[must_use]
    pub fn ctime_as_timeval(&self) -> libc::timeval {
        crate::os::new_timeval(self.ctime / 1_000_000, self.ctime % 1_000_000)
    }

    #[must_use]
    pub fn atime_as_timeval(&self) -> libc::timeval {
        crate::os::new_timeval(self.atime / 1_000_000, self.atime % 1_000_000)
    }

    #[must_use]
    pub fn mtime_as_timeval(&self) -> libc::timeval {
        crate::os::new_timeval(self.mtime / 1_000_000, self.mtime % 1_000_000)
    }

    #[must_use]
    pub fn get_utimes_timeval(&self) -> [libc::timeval; 2] {
        [self.atime_as_timeval(), self.mtime_as_timeval()]
    }
}

impl crate::fs::Hammer2InodeData {
    #[must_use]
    pub fn u_as<T>(&self) -> &T {
        crate::util::align_to(&self.u)
    }

    pub fn u_as_mut<T>(&mut self) -> &mut T {
        crate::util::align_to_mut(&mut self.u)
    }

    /// # Errors
    pub fn get_filename_string(&self) -> Result<String, std::str::Utf8Error> {
        let n = usize::from(self.meta.name_len);
        Ok(if n <= crate::fs::HAMMER2_INODE_MAXNAME {
            std::str::from_utf8(&self.filename[..n])?
        } else {
            ""
        }
        .to_string())
    }
}

impl crate::fs::Hammer2VolumeData {
    /// # Panics
    #[must_use]
    pub fn get_crc(&self, offset: u64, size: u64) -> u32 {
        let voldata = crate::util::any_as_u8_slice(self);
        let beg = offset.try_into().unwrap();
        let end = (offset + size).try_into().unwrap();
        icrc32::iscsi_crc32(&voldata[beg..end])
    }
}

impl crate::ioctl::IocPfs {
    pub fn copy_name(&mut self, name: &[u8]) {
        let n = if name.len() > self.name.len() {
            self.name.len()
        } else {
            name.len()
        };
        self.name[..n].copy_from_slice(&name[..n]);
    }
}

impl crate::ioctl::IocDestroy {
    pub fn copy_path(&mut self, path: &[u8]) {
        let n = if path.len() > self.path.len() {
            self.path.len()
        } else {
            path.len()
        };
        self.path[..n].copy_from_slice(&path[..n]);
    }
}

#[must_use]
pub fn media_as<T>(media: &[u8]) -> Vec<&T> {
    let x = std::mem::size_of::<T>();
    let n = media.len() / x;
    let mut v = vec![];
    for i in 0..n {
        v.push(crate::util::align_to::<T>(&media[i * x..(i + 1) * x]));
    }
    v
}

#[cfg(test)]
mod tests {
    macro_rules! eq {
        ($val: expr, $ptr: expr) => {
            let a = format!("{:?}", std::ptr::addr_of!($val));
            let b = format!("{:?}", std::ptr::from_ref($ptr));
            assert_eq!(a, b);
        };
    }

    #[test]
    fn test_blockref_embed_as() {
        let bref = crate::fs::Hammer2Blockref::new_empty();
        eq!(bref.embed, bref.embed_as::<crate::fs::Hammer2DirentHead>());
        eq!(
            bref.embed,
            bref.embed_as::<crate::fs::Hammer2BlockrefEmbedStats>()
        );
    }

    #[test]
    fn test_blockref_embed_as_mut() {
        let mut bref = crate::fs::Hammer2Blockref::new_empty();
        eq!(
            bref.embed,
            bref.embed_as_mut::<crate::fs::Hammer2DirentHead>()
        );
        eq!(
            bref.embed,
            bref.embed_as_mut::<crate::fs::Hammer2BlockrefEmbedStats>()
        );
    }

    #[test]
    fn test_blockref_check_as() {
        let bref = crate::fs::Hammer2Blockref::new_empty();
        eq!(
            bref.check,
            bref.check_as::<crate::fs::Hammer2BlockrefCheckIscsi>()
        );
        eq!(
            bref.check,
            bref.check_as::<crate::fs::Hammer2BlockrefCheckXxhash64>()
        );
        eq!(
            bref.check,
            bref.check_as::<crate::fs::Hammer2BlockrefCheckSha192>()
        );
        eq!(
            bref.check,
            bref.check_as::<crate::fs::Hammer2BlockrefCheckSha256>()
        );
        eq!(
            bref.check,
            bref.check_as::<crate::fs::Hammer2BlockrefCheckSha512>()
        );
        eq!(
            bref.check,
            bref.check_as::<crate::fs::Hammer2BlockrefCheckFreemap>()
        );
    }

    #[test]
    fn test_blockref_check_as_mut() {
        let mut bref = crate::fs::Hammer2Blockref::new_empty();
        eq!(
            bref.check,
            bref.check_as_mut::<crate::fs::Hammer2BlockrefCheckIscsi>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<crate::fs::Hammer2BlockrefCheckXxhash64>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<crate::fs::Hammer2BlockrefCheckSha192>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<crate::fs::Hammer2BlockrefCheckSha256>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<crate::fs::Hammer2BlockrefCheckSha512>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<crate::fs::Hammer2BlockrefCheckFreemap>()
        );
    }

    #[test]
    fn test_blockset_as_blockref() {
        let bset = crate::fs::Hammer2Blockset::new();
        eq!(bset, bset.as_blockref()[0]);
        eq!(bset.blockref[0], bset.as_blockref()[0]);
        eq!(bset.blockref[1], bset.as_blockref()[1]);
        eq!(bset.blockref[2], bset.as_blockref()[2]);
        eq!(bset.blockref[3], bset.as_blockref()[3]);
    }

    #[test]
    fn test_inode_meta_ctime_as_timeval() {
        let ipmeta = crate::fs::Hammer2InodeMeta {
            ctime: 1_000_001,
            ..Default::default()
        };
        assert_eq!(
            ipmeta.ctime_as_timeval(),
            libc::timeval {
                tv_sec: 1,
                tv_usec: 1
            }
        );
    }

    #[test]
    fn test_inode_meta_atime_as_timeval() {
        let ipmeta = crate::fs::Hammer2InodeMeta {
            atime: 1_000_001,
            ..Default::default()
        };
        assert_eq!(
            ipmeta.atime_as_timeval(),
            libc::timeval {
                tv_sec: 1,
                tv_usec: 1
            }
        );
    }

    #[test]
    fn test_inode_meta_mtime_as_timeval() {
        let ipmeta = crate::fs::Hammer2InodeMeta {
            mtime: 1_000_001,
            ..Default::default()
        };
        assert_eq!(
            ipmeta.mtime_as_timeval(),
            libc::timeval {
                tv_sec: 1,
                tv_usec: 1
            }
        );
    }

    #[test]
    fn test_inode_data_u_as() {
        let ipdata = crate::fs::Hammer2InodeData::new();
        eq!(ipdata.u, ipdata.u_as::<crate::fs::Hammer2Blockset>());
    }

    #[test]
    fn test_inode_data_u_as_mut() {
        let mut ipdata = crate::fs::Hammer2InodeData::new();
        eq!(ipdata.u, ipdata.u_as_mut::<crate::fs::Hammer2Blockset>());
    }

    #[test]
    fn test_inode_data_get_filename_string() {
        let ipdata = crate::fs::Hammer2InodeData::new();
        assert_eq!(
            match ipdata.get_filename_string() {
                Ok(v) => v,
                Err(e) => panic!("{e}"),
            },
            String::new()
        );

        for s in [
            String::new(),
            "A".to_string(),
            "A".repeat(crate::fs::HAMMER2_INODE_MAXNAME),
        ] {
            let mut ipdata = crate::fs::Hammer2InodeData::new();
            ipdata.meta.name_len = match s.len().try_into() {
                Ok(v) => v,
                Err(e) => panic!("{e}"),
            };
            ipdata.filename[..s.len()].copy_from_slice(s.as_bytes());
            assert_eq!(
                match ipdata.get_filename_string() {
                    Ok(v) => v,
                    Err(e) => panic!("{e}"),
                },
                s
            );
        }
    }
}
