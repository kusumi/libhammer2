use crate::fs;
use crate::ioctl;
use crate::util;

impl fs::Hammer2Blockref {
    #[must_use]
    pub fn embed_as<T>(&self) -> &T {
        util::align_to(&self.embed)
    }

    pub fn embed_as_mut<T>(&mut self) -> &mut T {
        util::align_to_mut(&mut self.embed)
    }

    #[must_use]
    pub fn check_as<T>(&self) -> &T {
        util::align_to(&self.check)
    }

    pub fn check_as_mut<T>(&mut self) -> &mut T {
        util::align_to_mut(&mut self.check)
    }
}

impl fs::Hammer2Blockset {
    #[must_use]
    pub fn as_blockref(&self) -> [&fs::Hammer2Blockref; fs::HAMMER2_SET_COUNT] {
        [
            &self.blockref[0],
            &self.blockref[1],
            &self.blockref[2],
            &self.blockref[3],
        ]
    }
}

impl fs::Hammer2InodeData {
    #[must_use]
    pub fn u_as<T>(&self) -> &T {
        util::align_to(&self.u)
    }

    pub fn u_as_mut<T>(&mut self) -> &mut T {
        util::align_to_mut(&mut self.u)
    }

    /// # Panics
    #[must_use]
    pub fn get_filename_string(&self) -> String {
        let n = usize::from(self.meta.name_len);
        if n <= fs::HAMMER2_INODE_MAXNAME {
            std::str::from_utf8(&self.filename[..n]).unwrap()
        } else {
            ""
        }
        .to_string()
    }
}

impl fs::Hammer2VolumeData {
    /// # Panics
    #[must_use]
    pub fn get_crc(&self, offset: u64, size: u64) -> u32 {
        let voldata = util::any_as_u8_slice(self);
        let beg = offset.try_into().unwrap();
        let end = (offset + size).try_into().unwrap();
        icrc32::iscsi_crc32(&voldata[beg..end])
    }
}

impl ioctl::Hammer2IocPfs {
    pub fn copy_name(&mut self, name: &[u8]) {
        let n = if name.len() > self.name.len() {
            self.name.len()
        } else {
            name.len()
        };
        self.name[..n].copy_from_slice(&name[..n]);
    }
}

impl ioctl::Hammer2IocDestroy {
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
        v.push(util::align_to::<T>(&media[i * x..(i + 1) * x]));
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
        let bref = super::fs::Hammer2Blockref::new_empty();
        eq!(bref.embed, bref.embed_as::<super::fs::Hammer2DirentHead>());
        eq!(
            bref.embed,
            bref.embed_as::<super::fs::Hammer2BlockrefEmbedStats>()
        );
    }

    #[test]
    fn test_blockref_embed_as_mut() {
        let mut bref = super::fs::Hammer2Blockref::new_empty();
        eq!(
            bref.embed,
            bref.embed_as_mut::<super::fs::Hammer2DirentHead>()
        );
        eq!(
            bref.embed,
            bref.embed_as_mut::<super::fs::Hammer2BlockrefEmbedStats>()
        );
    }

    #[test]
    fn test_blockref_check_as() {
        let bref = super::fs::Hammer2Blockref::new_empty();
        eq!(
            bref.check,
            bref.check_as::<super::fs::Hammer2BlockrefCheckIscsi>()
        );
        eq!(
            bref.check,
            bref.check_as::<super::fs::Hammer2BlockrefCheckXxhash64>()
        );
        eq!(
            bref.check,
            bref.check_as::<super::fs::Hammer2BlockrefCheckSha192>()
        );
        eq!(
            bref.check,
            bref.check_as::<super::fs::Hammer2BlockrefCheckSha256>()
        );
        eq!(
            bref.check,
            bref.check_as::<super::fs::Hammer2BlockrefCheckSha512>()
        );
        eq!(
            bref.check,
            bref.check_as::<super::fs::Hammer2BlockrefCheckFreemap>()
        );
    }

    #[test]
    fn test_blockref_check_as_mut() {
        let mut bref = super::fs::Hammer2Blockref::new_empty();
        eq!(
            bref.check,
            bref.check_as_mut::<super::fs::Hammer2BlockrefCheckIscsi>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<super::fs::Hammer2BlockrefCheckXxhash64>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<super::fs::Hammer2BlockrefCheckSha192>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<super::fs::Hammer2BlockrefCheckSha256>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<super::fs::Hammer2BlockrefCheckSha512>()
        );
        eq!(
            bref.check,
            bref.check_as_mut::<super::fs::Hammer2BlockrefCheckFreemap>()
        );
    }

    #[test]
    fn test_blockset_as_blockref() {
        let bset = super::fs::Hammer2Blockset::new();
        eq!(bset, bset.as_blockref()[0]);
        eq!(bset.blockref[0], bset.as_blockref()[0]);
        eq!(bset.blockref[1], bset.as_blockref()[1]);
        eq!(bset.blockref[2], bset.as_blockref()[2]);
        eq!(bset.blockref[3], bset.as_blockref()[3]);
    }

    #[test]
    fn test_inode_data_u_as() {
        let ipdata = super::fs::Hammer2InodeData::new();
        eq!(ipdata.u, ipdata.u_as::<super::fs::Hammer2Blockset>());
    }

    #[test]
    fn test_inode_data_u_as_mut() {
        let mut ipdata = super::fs::Hammer2InodeData::new();
        eq!(ipdata.u, ipdata.u_as_mut::<super::fs::Hammer2Blockset>());
    }

    #[test]
    fn test_inode_data_get_filename_string() {
        let ipdata = super::fs::Hammer2InodeData::new();
        assert_eq!(ipdata.get_filename_string(), String::new());

        for s in [
            String::new(),
            "A".to_string(),
            "A".repeat(super::fs::HAMMER2_INODE_MAXNAME),
        ] {
            let mut ipdata = super::fs::Hammer2InodeData::new();
            ipdata.meta.name_len = s.len().try_into().unwrap();
            ipdata.filename[..s.len()].copy_from_slice(s.as_bytes());
            assert_eq!(ipdata.get_filename_string(), s);
        }
    }
}
