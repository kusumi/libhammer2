#[derive(Debug, Default)]
pub(crate) struct XopHeader {
    cids: Vec<crate::chain::Cid>,
    pub(crate) inum1: u64,
    pub(crate) name1: String,
}

impl XopHeader {
    pub(crate) fn new(inum1: u64, name1: &str) -> Self {
        Self {
            inum1,
            name1: name1.to_string(),
            ..Default::default()
        }
    }

    pub(crate) fn new_from_inum(inum1: u64) -> Self {
        Self {
            inum1,
            ..Default::default()
        }
    }

    pub(crate) fn dummy_new(cid: crate::chain::Cid) -> Self {
        Self {
            cids: vec![cid],
            ..Default::default()
        }
    }

    pub(crate) fn feed(&mut self, cid: crate::chain::Cid) {
        if cid != crate::chain::CID_NONE {
            self.cids.push(cid);
        }
    }

    pub(crate) fn collect(&self) -> nix::Result<crate::chain::Cid> {
        if self.cids.is_empty() {
            Err(nix::errno::Errno::ENOENT)
        } else {
            Ok(self.cids[0])
        }
    }

    pub(crate) fn collect_all(&self) -> nix::Result<Vec<crate::chain::Cid>> {
        if self.cids.is_empty() {
            Err(nix::errno::Errno::ENOENT)
        } else {
            Ok(self.cids.clone())
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct XopNresolve {
    pub(crate) head: XopHeader,
}

impl XopNresolve {
    pub(crate) fn new(inum1: u64, name1: &str) -> Self {
        Self {
            head: XopHeader::new(inum1, name1),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct XopReaddir {
    pub(crate) head: XopHeader,
    pub(crate) lkey: u64,
}

impl XopReaddir {
    pub(crate) fn new(inum1: u64, lkey: u64) -> Self {
        Self {
            head: XopHeader::new_from_inum(inum1),
            lkey,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct XopBmap {
    pub(crate) head: XopHeader,
    pub(crate) lbn: u64,
    pub(crate) offset: u64,
}

impl XopBmap {
    pub(crate) fn new(inum1: u64, lbn: u64) -> Self {
        Self {
            head: XopHeader::new_from_inum(inum1),
            lbn,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct XopRead {
    pub(crate) head: XopHeader,
    pub(crate) lbase: u64,
}

impl XopRead {
    pub(crate) fn new(inum1: u64, lbase: u64) -> Self {
        Self {
            head: XopHeader::new_from_inum(inum1),
            lbase,
        }
    }
}
