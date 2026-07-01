pub const INUM_SUP_ROOT: u64 = 0;
pub const INUM_PFS_ROOT: u64 = 1;

pub const PFS_LABEL_BOOT: &str = "BOOT";
pub const PFS_LABEL_ROOT: &str = "ROOT";
pub const PFS_LABEL_DATA: &str = "DATA";
pub const PFS_LABEL_LOCAL: &str = "LOCAL";

pub const PFS_LABEL_DEFAULT: &str = PFS_LABEL_DATA;

#[derive(Debug)]
pub struct Inode {
    pub(crate) meta: crate::fs::Hammer2InodeMeta,
    pub(crate) cid: crate::chain::Cid, // Rust
    refs: usize,
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

    /// # Errors
    pub fn get(&mut self) -> nix::Result<()> {
        self.refs = if let Some(v) = self.refs.checked_add(1) {
            v
        } else {
            log::error!("cid {} refs {} meta {:#?}", self.cid, self.refs, self.meta);
            return Err(nix::errno::Errno::EINVAL);
        };
        Ok(())
    }

    /// # Errors
    pub fn put(&mut self) -> nix::Result<()> {
        self.refs = if let Some(v) = self.refs.checked_sub(1) {
            v
        } else {
            log::error!("cid {} refs {} meta {:#?}", self.cid, self.refs, self.meta);
            return Err(nix::errno::Errno::EINVAL);
        };
        Ok(())
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
        assert!(ip.get().is_ok());
        assert_eq!(ip.refs, 1);
        assert!(ip.get().is_ok());
        assert_eq!(ip.refs, 2);
        assert!(ip.put().is_ok());
        assert_eq!(ip.refs, 1);
        assert!(ip.put().is_ok());
        assert_eq!(ip.refs, 0);
        assert!(ip.put().is_err());
        assert_eq!(ip.refs, 0);
        assert!(ip.put().is_err());
    }
}
