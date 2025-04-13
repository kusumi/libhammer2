use std::fmt;

pub(crate) fn conv_offset_to_radix(x: u64) -> u8 {
    (x & crate::fs::HAMMER2_OFF_MASK_RADIX).try_into().unwrap()
}

pub(crate) fn conv_offset_to_raw_data_off(x: u64) -> u64 {
    x & !crate::fs::HAMMER2_OFF_MASK_RADIX
}

impl crate::fs::Hammer2Blockref {
    pub(crate) fn is_node_type(&self) -> bool {
        self.typ == crate::fs::HAMMER2_BREF_TYPE_INDIRECT
            || self.typ == crate::fs::HAMMER2_BREF_TYPE_FREEMAP_NODE
    }

    #[allow(dead_code)]
    pub(crate) fn is_leaf_type(&self) -> bool {
        self.typ == crate::fs::HAMMER2_BREF_TYPE_DATA
            || self.typ == crate::fs::HAMMER2_BREF_TYPE_FREEMAP_LEAF
    }

    #[must_use]
    pub fn get_radix(&self) -> u8 {
        conv_offset_to_radix(self.data_off)
    }

    #[must_use]
    pub fn get_raw_data_off(&self) -> u64 {
        conv_offset_to_raw_data_off(self.data_off)
    }

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

impl fmt::Display for crate::fs::Hammer2Blockref {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {:016x} {:016x}/{} meth {:02x} leaf {} mir {:016x} mod {:016x}",
            crate::subs::get_blockref_type_string(self.typ),
            self.data_off,
            self.key,
            self.keybits,
            self.methods,
            self.leaf_count,
            self.mirror_tid,
            self.modify_tid
        )
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
    pub fn has_direct_data(&self) -> bool {
        (self.op_flags & crate::fs::HAMMER2_OPFLAG_DIRECTDATA) != 0
    }

    #[must_use]
    pub fn is_pfs_root(&self) -> bool {
        (self.op_flags & crate::fs::HAMMER2_OPFLAG_PFSROOT) != 0
    }

    #[must_use]
    pub fn is_sup_root(&self) -> bool {
        self.pfs_type == crate::fs::HAMMER2_PFSTYPE_SUPROOT
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.is_pfs_root() || self.is_sup_root()
    }

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
    pub fn get_filename_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(
            self.filename[..std::cmp::min(
                usize::from(self.meta.name_len),
                crate::fs::HAMMER2_INODE_MAXNAME,
            )]
                .to_vec(),
        )
    }
}

impl fmt::Display for crate::fs::Hammer2InodeData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} clid {} fsid {}",
            if let Ok(v) = self.get_filename_string() {
                v
            } else {
                "???".to_string()
            },
            crate::subs::get_uuid_string_from_bytes(&self.meta.pfs_clid),
            crate::subs::get_uuid_string_from_bytes(&self.meta.pfs_fsid)
        )
    }
}

impl crate::fs::Hammer2VolumeData {
    /// # Panics
    #[must_use]
    pub fn is_hbo(&self) -> bool {
        match self.magic {
            crate::fs::HAMMER2_VOLUME_ID_HBO => true,
            crate::fs::HAMMER2_VOLUME_ID_ABO => false,
            _ => panic!("{:x}", self.magic),
        }
    }

    /// # Panics
    #[must_use]
    pub fn get_crc(&self, offset: u64, size: u64) -> u32 {
        let voldata = crate::util::any_as_u8_slice(self);
        let beg = offset.try_into().unwrap();
        let end = (offset + size).try_into().unwrap();
        icrc32::iscsi_crc32(&voldata[beg..end])
    }
}

fn copy_bytes(dst: &mut [u8], src: &[u8]) {
    dst.fill(0);
    let n = std::cmp::min(src.len(), dst.len());
    dst[..n].copy_from_slice(&src[..n]);
}

impl crate::ioctl::IocPfs {
    /// # Errors
    pub fn get_name(&self) -> nix::Result<Vec<u8>> {
        match crate::util::bin_to_string(&self.name) {
            Ok(v) => Ok(self.name[..v.len()].to_vec()),
            Err(e) => {
                log::error!("{e}");
                Err(nix::errno::Errno::EINVAL)
            }
        }
    }

    /// # Errors
    pub fn get_name_lhc(&self) -> nix::Result<u64> {
        Ok(crate::subs::dirhash(&self.get_name()?))
    }

    pub fn copy_name(&mut self, name: &[u8]) {
        copy_bytes(&mut self.name, name);
    }
}

impl crate::ioctl::IocDestroy {
    pub fn copy_path(&mut self, path: &[u8]) {
        copy_bytes(&mut self.path, path);
    }
}

impl crate::ioctl::IocVolume {
    pub fn copy_path(&mut self, path: &[u8]) {
        copy_bytes(&mut self.path, path);
    }
}

impl crate::ioctl::IocVolumeList {
    pub fn copy_pfs_name(&mut self, pfs_name: &[u8]) {
        copy_bytes(&mut self.pfs_name, pfs_name);
    }
}

impl crate::ioctl::IocVolume2 {
    pub fn copy_path(&mut self, path: &[u8]) {
        copy_bytes(&mut self.path, path);
    }
}

impl crate::ioctl::IocVolumeList2 {
    pub fn copy_pfs_name(&mut self, pfs_name: &[u8]) {
        copy_bytes(&mut self.pfs_name, pfs_name);
    }
}

impl crate::hammer2::Hammer2 {
    #[must_use]
    pub fn get_volumes(&self) -> Vec<&crate::volume::Volume> {
        self.fso.get_volumes()
    }

    #[must_use]
    pub fn get_volume_data(&self) -> &crate::fs::Hammer2VolumeData {
        &self.voldata
    }

    #[must_use]
    pub fn get_label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub fn get_chain(&self, cid: crate::chain::Cid) -> Option<&crate::chain::Chain> {
        self.cmap.get(&cid)
    }

    #[must_use]
    pub fn get_inode(&self, inum: u64) -> Option<&crate::inode::Inode> {
        self.nmap.get(&inum)
    }

    pub fn get_inode_mut(&mut self, inum: u64) -> Option<&mut crate::inode::Inode> {
        self.nmap.get_mut(&inum)
    }

    pub(crate) fn alloc_cid(&mut self) -> nix::Result<crate::chain::Cid> {
        assert!(self.imap.next >= crate::chain::CID_CHAIN_OFFSET);
        assert_ne!(self.imap.max, 0);
        let cid = match self.opt.cidalloc {
            crate::option::CidAllocMode::Linear => self.alloc_cidmap_linear()?,
            crate::option::CidAllocMode::Bitmap => self.alloc_cidmap_bitmap()?,
        };
        assert_ne!(cid, crate::chain::CID_NONE);
        assert_ne!(cid, crate::chain::CID_VCHAIN);
        assert_ne!(cid, crate::chain::CID_FCHAIN);
        Ok(cid)
    }

    fn alloc_cidmap_linear(&mut self) -> nix::Result<crate::chain::Cid> {
        if self.imap.next > self.imap.max {
            return Err(nix::errno::Errno::ENOSPC);
        }
        let cid = self.imap.next;
        self.imap.next += 1;
        Ok(cid)
    }

    fn alloc_cidmap_bitmap(&self) -> nix::Result<crate::chain::Cid> {
        Err(nix::errno::Errno::EOPNOTSUPP) // XXX
    }

    /// # Errors
    pub fn readlinkx(&mut self, inum: u64) -> crate::Result<String> {
        let mut buf = vec![0; crate::fs::HAMMER2_INODE_MAXNAME];
        self.readlink(inum, &mut buf)?;
        match crate::util::bin_to_string(&buf) {
            Ok(v) => Ok(v),
            Err(e) => {
                log::error!("{e}");
                Err(nix::errno::Errno::EINVAL.into())
            }
        }
    }

    /// # Errors
    /// # Panics
    pub fn preadx(&mut self, inum: u64, size: u64, offset: u64) -> crate::Result<Vec<u8>> {
        let mut buf = vec![0; size.try_into().unwrap()];
        let n = self.pread(inum, &mut buf, offset)?;
        Ok(buf[..n.try_into().unwrap()].to_vec())
    }

    /// # Errors
    pub fn read_all(&mut self, inum: u64) -> crate::Result<Vec<u8>> {
        self.preadx(inum, self.stat(inum)?.st_size, 0)
    }
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
        assert_eq!(bset.blockref.len(), crate::fs::HAMMER2_SET_COUNT);
        assert_eq!(bset.as_blockref().len(), crate::fs::HAMMER2_SET_COUNT);
        eq!(bset, bset.as_blockref()[0]);
        eq!(bset.blockref[0], bset.as_blockref()[0]);
        eq!(bset.blockref[1], bset.as_blockref()[1]);
        eq!(bset.blockref[2], bset.as_blockref()[2]);
        eq!(bset.blockref[3], bset.as_blockref()[3]);
    }

    #[test]
    fn test_inode_meta_has_direct_data() {
        let ipmeta = crate::fs::Hammer2InodeMeta {
            ..Default::default()
        };
        assert!(!ipmeta.has_direct_data());
        let ipmeta = crate::fs::Hammer2InodeMeta {
            op_flags: crate::fs::HAMMER2_OPFLAG_PFSROOT,
            ..Default::default()
        };
        assert!(!ipmeta.has_direct_data());
        let ipmeta = crate::fs::Hammer2InodeMeta {
            op_flags: crate::fs::HAMMER2_OPFLAG_DIRECTDATA,
            ..Default::default()
        };
        assert!(ipmeta.has_direct_data());
        let ipmeta = crate::fs::Hammer2InodeMeta {
            op_flags: crate::fs::HAMMER2_OPFLAG_DIRECTDATA | crate::fs::HAMMER2_OPFLAG_PFSROOT,
            ..Default::default()
        };
        assert!(ipmeta.has_direct_data());
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
