pub const INUM_SUP_ROOT: u64 = 0;
pub const INUM_PFS_ROOT: u64 = 1;

pub(crate) const DEFAULT_PFS_LABEL: &str = "DATA";

#[derive(Debug)]
pub struct Inode {
    pub(crate) meta: crate::fs::Hammer2InodeMeta,
    pub(crate) cid: crate::chain::Cid, // Rust
    refs: isize,
}

impl Inode {
    pub(crate) fn new(meta: &crate::fs::Hammer2InodeMeta, cid: crate::chain::Cid) -> Self {
        Self {
            meta: *meta,
            cid,
            refs: 0,
        }
    }

    pub(crate) fn new_empty() -> Self {
        Self::new(&crate::fs::Hammer2InodeMeta::new(), crate::chain::CID_NONE)
    }

    #[must_use]
    pub fn get_meta(&self) -> &crate::fs::Hammer2InodeMeta {
        &self.meta
    }

    /// # Panics
    pub fn get(&mut self) {
        assert!(
            self.get_impl().is_ok(),
            "cid {} refs {} meta {:#?}",
            self.cid,
            self.refs,
            self.meta
        );
    }

    fn get_impl(&mut self) -> nix::Result<()> {
        if self.refs < isize::MAX {
            self.refs += 1;
            Ok(())
        } else {
            Err(nix::errno::Errno::EINVAL)
        }
    }

    /// # Panics
    pub fn put(&mut self) {
        assert!(
            self.put_impl().is_ok(),
            "cid {} refs {} meta {:#?}",
            self.cid,
            self.refs,
            self.meta
        );
    }

    fn put_impl(&mut self) -> nix::Result<()> {
        if self.refs > 0 {
            self.refs -= 1;
            Ok(())
        } else {
            Err(nix::errno::Errno::EINVAL)
        }
    }

    #[must_use]
    pub fn is_directory(&self) -> bool {
        self.meta.typ == crate::fs::HAMMER2_OBJTYPE_DIRECTORY
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_inode_get_put() {
        let mut ip = super::Inode::new_empty();
        assert_eq!(ip.refs, 0);
        assert!(ip.get_impl().is_ok());
        assert_eq!(ip.refs, 1);
        assert!(ip.get_impl().is_ok());
        assert_eq!(ip.refs, 2);
        assert!(ip.put_impl().is_ok());
        assert_eq!(ip.refs, 1);
        assert!(ip.put_impl().is_ok());
        assert_eq!(ip.refs, 0);
        assert!(ip.put_impl().is_err());
        assert_eq!(ip.refs, 0);
        assert!(ip.put_impl().is_err());
    }
}
