macro_rules! get_chain {
    ($pmp:expr, $cid:expr) => {
        $pmp.cmap.get($cid).unwrap()
    };
}

macro_rules! get_chain_mut {
    ($pmp:expr, $cid:expr) => {
        $pmp.cmap.get_mut($cid).unwrap()
    };
}

macro_rules! get_inode {
    ($pmp:expr, $inum:expr) => {
        $pmp.nmap.get($inum).unwrap()
    };
}

macro_rules! get_inode_mut {
    ($pmp:expr, $inum:expr) => {
        $pmp.nmap.get_mut($inum).unwrap()
    };
}

const HAMMER2_COMPAT: &str = "HAMMER2_COMPAT";

const LOOKUP_ALWAYS: u32 = 0x0000_0800; // resolve data

pub const RESOLVE_MAYBE: u32 = 2;
pub const RESOLVE_ALWAYS: u32 = 3;
const RESOLVE_MASK: u32 = 0x0F;

#[derive(Debug)]
pub struct Dirent {
    pub inum: u64,
    pub typ: u8,
    pub name: String,
}

impl Dirent {
    fn new(inum: u64, typ: u8, name: &str) -> Self {
        Self {
            inum,
            typ,
            name: name.to_string(),
        }
    }
}

#[cfg(target_os = "linux")]
pub type StatMode = u32;
#[cfg(not(target_os = "linux"))] // FreeBSD
pub type StatMode = u16;

#[derive(Debug)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u32,
    pub st_mode: StatMode,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u32,
    pub st_size: u64,
    pub st_blksize: u32,
    pub st_blocks: u64,
    pub st_atime: u64,
    pub st_mtime: u64,
    pub st_ctime: u64,
}

#[derive(Debug)]
pub struct StatFs {
    pub f_bsize: u32,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_namelen: u32,
    pub f_frsize: u32,
}

#[derive(Debug, Default)]
pub(crate) struct CidMap {
    pub(crate) next: crate::chain::Cid,
    pub(crate) max: crate::chain::Cid,
    pub(crate) pool: Vec<crate::chain::Cid>,
    pub(crate) chunk: libfs::bitmap::Bitmap,
}

impl CidMap {
    fn new() -> Self {
        Self {
            next: crate::chain::CID_CHAIN_OFFSET,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct Hammer2 {
    pub(crate) opt: crate::option::Opt,
    pub(crate) fso: crate::ondisk::Ondisk,
    pub(crate) voldata: crate::fs::Hammer2VolumeData,
    pub(crate) label: String,
    pub(crate) imap: CidMap,
    pub(crate) cmap: std::collections::HashMap<crate::chain::Cid, crate::chain::Chain>,
    pub(crate) nmap: std::collections::HashMap<u64, crate::inode::Inode>,
}

impl Drop for Hammer2 {
    fn drop(&mut self) {
        if !self.cmap.is_empty() {
            log::debug!("unmount {} on drop", self.label);
            self.unmount().unwrap();
        }
    }
}

impl Hammer2 {
    fn new(fso: crate::ondisk::Ondisk, opt: crate::option::Opt) -> crate::Result<Self> {
        let voldata = fso.read_root_volume_data()?;
        Ok(Self {
            opt,
            fso,
            voldata,
            label: String::new(),
            nmap: std::collections::HashMap::new(),
            imap: CidMap::new(),
            cmap: std::collections::HashMap::new(),
        })
    }

    fn add_chain(
        &mut self,
        pcid: crate::chain::Cid,
        mut chain: crate::chain::Chain,
    ) -> nix::Result<()> {
        if let Some(chain) = self.cmap.get(&chain.cid) {
            log::error!("collision {}", chain.bref);
            return Err(nix::errno::Errno::EEXIST);
        }
        assert_ne!(chain.cid, crate::chain::CID_NONE);
        assert_ne!(chain.cid, crate::chain::CID_VCHAIN);
        assert_ne!(chain.cid, crate::chain::CID_FCHAIN);
        assert_eq!(chain.pcid, crate::chain::CID_NONE);
        chain.pcid = pcid;
        get_chain_mut!(self, &pcid).add_child(&chain);
        assert!(self.cmap.insert(chain.cid, chain).is_none());
        Ok(())
    }

    pub(crate) fn remove_chain(
        &mut self,
        pcid: crate::chain::Cid,
        cid: crate::chain::Cid,
    ) -> nix::Result<crate::chain::Chain> {
        assert_ne!(cid, crate::chain::CID_NONE);
        assert_ne!(cid, crate::chain::CID_VCHAIN);
        assert_ne!(cid, crate::chain::CID_FCHAIN);
        assert_ne!(pcid, crate::chain::CID_NONE);
        get_chain_mut!(self, &pcid).remove_child(cid)?;
        match self.cmap.remove(&cid) {
            Some(chain) => Ok(chain),
            None => Err(nix::errno::Errno::ENOENT),
        }
    }

    fn clear_chain(&mut self) -> nix::Result<()> {
        self.clear_chain_impl(crate::chain::CID_VCHAIN)?;
        self.clear_chain_impl(crate::chain::CID_FCHAIN)
    }

    fn clear_chain_impl(&mut self, cid: crate::chain::Cid) -> nix::Result<()> {
        while let Some(ccid) = get_chain!(self, &cid).get_first_child() {
            self.clear_chain_impl(ccid)?;
            let chain = self.remove_chain(cid, ccid)?;
            if chain.bref.typ == crate::fs::HAMMER2_BREF_TYPE_INODE {
                let ipdata = chain.as_inode_data();
                if ipdata.meta.is_pfs_root() {
                    match ipdata.get_filename_string() {
                        Ok(s) => {
                            if s == self.label {
                                assert_eq!(
                                    self.remove_inode(ipdata.meta.inum)?.meta.inum,
                                    ipdata.meta.inum
                                );
                            }
                        }
                        Err(e) => {
                            log::error!("{e}");
                            return Err(nix::errno::Errno::EINVAL);
                        }
                    }
                } else {
                    assert_eq!(
                        self.remove_inode(ipdata.meta.inum)?.meta.inum,
                        ipdata.meta.inum
                    );
                }
            }
        }
        Ok(())
    }

    fn set_chain(
        &mut self,
        pcid: crate::chain::Cid,
        bref: &crate::fs::Hammer2Blockref,
        how: u32,
    ) -> crate::Result<crate::chain::Cid> {
        let chain = crate::chain::Chain::new(bref, self.alloc_cid()?)?;
        let cid = chain.cid;
        self.add_chain(pcid, chain)?;
        if let Err(e) = self.load_chain(cid, how) {
            self.remove_chain(pcid, cid)?;
            return Err(e);
        }
        Ok(cid)
    }

    fn load_chain(&mut self, cid: crate::chain::Cid, how: u32) -> crate::Result<()> {
        // Do we have to resolve the data?  This is generally only
        // applicable to HAMMER2_BREF_TYPE_DATA which is special-cased.
        // Other blockref types expects the data to be there.
        match how & RESOLVE_MASK {
            RESOLVE_MAYBE => {
                if get_chain!(self, &cid).bref.typ == crate::fs::HAMMER2_BREF_TYPE_DATA {
                    return Ok(());
                }
            }
            RESOLVE_ALWAYS | 0 => (), // 0 is effectively same as RESOLVE_ALWAYS
            _ => {
                log::error!("bad how flag {how:x}");
                return Err(nix::errno::Errno::EINVAL.into());
            }
        }
        let chain = get_chain_mut!(self, &cid);
        if chain.has_data() {
            return Ok(());
        }
        let offset = chain.bref.get_raw_data_off();
        if offset == 0 {
            return Ok(());
        }
        let b = self
            .fso
            .get_volume_mut(offset)
            .ok_or(nix::errno::Errno::ENODEV)?
            .preadx(chain.get_bytes(), offset)?;
        if !chain.test_check(&b)? {
            return Err(nix::errno::Errno::EINVAL.into());
        }
        match chain.bref.typ {
            crate::fs::HAMMER2_BREF_TYPE_INODE
            | crate::fs::HAMMER2_BREF_TYPE_INDIRECT
            | crate::fs::HAMMER2_BREF_TYPE_DATA
            | crate::fs::HAMMER2_BREF_TYPE_FREEMAP_NODE
            | crate::fs::HAMMER2_BREF_TYPE_FREEMAP_LEAF => {
                chain.set_data(b);
            }
            crate::fs::HAMMER2_BREF_TYPE_DIRENT => {
                assert_ne!(chain.get_bytes(), 0);
                chain.set_data(b);
            }
            crate::fs::HAMMER2_BREF_TYPE_FREEMAP | crate::fs::HAMMER2_BREF_TYPE_VOLUME => {
                log::error!("unresolved volume header");
                return Err(nix::errno::Errno::EINVAL.into());
            }
            _ => {
                log::error!("bad blockref type {}", chain.bref.typ);
                return Err(nix::errno::Errno::EINVAL.into());
            }
        }
        Ok(())
    }

    fn repparent_chain(
        &mut self,
        cid: crate::chain::Cid,
        how: u32,
    ) -> crate::Result<crate::chain::Cid> {
        let pcid = get_chain!(self, &cid).pcid;
        assert_ne!(pcid, crate::chain::CID_NONE);
        self.load_chain(pcid, how)?;
        Ok(pcid)
    }

    /// # Errors
    /// # Panics
    pub fn lookup_chain(
        &mut self,
        pcid: crate::chain::Cid,
        key_beg: u64,
        key_end: u64,
        flags: u32,
    ) -> crate::Result<(crate::chain::Cid, crate::chain::Cid, u64)> {
        let (how_maybe, how) = if (flags & LOOKUP_ALWAYS) != 0 {
            (RESOLVE_ALWAYS, RESOLVE_ALWAYS)
        } else {
            (RESOLVE_MAYBE, RESOLVE_MAYBE)
        };
        // Recurse parent upward if necessary until the parent completely
        // encloses the key range or we hit the inode.
        let mut pchain = get_chain!(self, &pcid);
        while pchain.bref.is_node_type() {
            let scan_beg = pchain.bref.key;
            let scan_end = scan_beg + (1 << pchain.bref.keybits) - 1;
            // always !CHAIN_DELETED even if existed
            if key_beg >= scan_beg && key_end <= scan_end {
                break;
            }
            let pcid = self.repparent_chain(pchain.cid, how_maybe)?;
            pchain = get_chain!(self, &pcid);
        }
        let mut pcid = pchain.cid;
        assert_ne!(pcid, crate::chain::CID_NONE);
        let mut key_beg = key_beg;
        for _ in 0..300_000 {
            let t = self.lookup_chain_impl(pcid, key_beg, key_end, how_maybe, how)?;
            pcid = t.0;
            assert_ne!(pcid, crate::chain::CID_NONE);
            key_beg = t.3;
            let (cid, key_next) = (t.1, t.2);
            if key_beg == u64::MAX {
                return Ok((pcid, cid, key_next));
            }
            assert_eq!(cid, crate::chain::CID_NONE);
            assert_eq!(key_next, u64::MAX);
            assert_ne!(key_beg, u64::MAX); // re-lookup required
        }
        log::error!("maxloops");
        Err(nix::errno::Errno::E2BIG.into())
    }

    fn lookup_chain_impl(
        &mut self,
        pcid: crate::chain::Cid,
        key_beg: u64,
        key_end: u64,
        how_maybe: u32,
        how: u32,
    ) -> crate::Result<(crate::chain::Cid, crate::chain::Cid, u64, u64)> {
        let pchain = get_chain!(self, &pcid);
        if pchain.bref.typ == crate::fs::HAMMER2_BREF_TYPE_INODE {
            // Special shortcut for embedded data returns the inode
            // itself.  Callers must detect this condition and access
            // the embedded data (the strategy code does this for us).
            //
            // This is only applicable to regular files and softlinks.
            if pchain.as_inode_data().meta.has_direct_data() {
                self.load_chain(pcid, RESOLVE_ALWAYS)?;
                return Ok((pcid, get_chain!(self, &pcid).cid, key_end + 1, u64::MAX));
            }
        }
        get_chain_mut!(self, &pcid).count_blockref()?;
        // Combined search.
        let (cid, i, key_next) = self.combined_find_chain(pcid, key_beg, key_end)?;
        // Exhausted parent chain, iterate.
        if cid == crate::chain::CID_NONE && i == usize::MAX {
            // Short cut single-key case.
            if key_beg == key_end {
                return Ok((pcid, crate::chain::CID_NONE, key_next, u64::MAX));
            }
            let pchain = get_chain!(self, &pcid);
            // Stop if we reached the end of the iteration.
            if !pchain.bref.is_node_type() {
                return Ok((pcid, crate::chain::CID_NONE, key_next, u64::MAX));
            }
            // Calculate next key, stop if we reached the end of the
            // iteration, otherwise go up one level and loop.
            let key_beg = pchain.bref.key + (1 << pchain.bref.keybits);
            if key_beg == 0 || key_beg > key_end {
                return Ok((pcid, crate::chain::CID_NONE, key_next, u64::MAX));
            }
            return Ok((
                self.repparent_chain(pchain.cid, how_maybe)?,
                crate::chain::CID_NONE,
                u64::MAX,
                key_beg,
            ));
        }
        let cid = if cid == crate::chain::CID_NONE {
            // Selected from blockref.
            let bref = *get_chain!(self, &pcid).as_blockref()?[i];
            if bref.is_node_type() {
                self.set_chain(pcid, &bref, how_maybe)
            } else {
                self.set_chain(pcid, &bref, how)
            }?
        } else {
            // Selected from in-memory chain.
            if get_chain!(self, &cid).bref.is_node_type() {
                self.load_chain(cid, how_maybe)?;
            } else {
                self.load_chain(cid, how)?;
            }
            assert_eq!(get_chain!(self, &cid).pcid, get_chain!(self, &pcid).cid);
            cid
        };
        assert_ne!(cid, crate::chain::CID_NONE);
        // If the chain element is an indirect block it becomes the new
        // parent and we loop on it.
        if get_chain!(self, &cid).bref.is_node_type() {
            return Ok((cid, crate::chain::CID_NONE, u64::MAX, key_beg));
        }
        Ok((pcid, cid, key_next, u64::MAX))
    }

    /// # Errors
    pub fn get_next_chain(
        &mut self,
        pcid: crate::chain::Cid,
        cid: crate::chain::Cid,
        key_end: u64,
        flags: u32,
    ) -> crate::Result<(crate::chain::Cid, crate::chain::Cid, u64)> {
        let pchain = get_chain!(self, &pcid);
        // Calculate the next index and recalculate the parent if necessary.
        let (pcid, key_beg) = if cid != crate::chain::CID_NONE {
            // chain invalid past this point, but we can still do a
            // cid comparison w/parent.
            //
            // Any scan where the lookup returned degenerate data embedded
            // in the inode has an invalid index and must terminate.
            if cid == pcid {
                return Ok((pcid, crate::chain::CID_NONE, u64::MAX));
            }
            let chain = get_chain!(self, &cid);
            let key_beg = chain.bref.key + (1 << chain.bref.keybits);
            if key_beg == 0 || key_beg > key_end {
                return Ok((pcid, crate::chain::CID_NONE, u64::MAX));
            }
            (pcid, key_beg)
        } else if !pchain.bref.is_node_type() {
            // We reached the end of the iteration.
            return Ok((pcid, crate::chain::CID_NONE, u64::MAX));
        } else {
            // Continue iteration with next parent unless the current
            // parent covers the range.
            //
            // (This also handles the case of a deleted, empty indirect
            // node).
            let key_beg = pchain.bref.key + (1 << pchain.bref.keybits);
            if key_beg == 0 || key_beg > key_end {
                return Ok((pcid, crate::chain::CID_NONE, u64::MAX));
            }
            (self.repparent_chain(pchain.cid, RESOLVE_MAYBE)?, key_beg)
        };
        // And execute.
        self.lookup_chain(pcid, key_beg, key_end, flags)
    }

    fn combined_find_chain(
        &mut self,
        pcid: crate::chain::Cid,
        key_beg: u64,
        key_end: u64,
    ) -> nix::Result<(u64, usize, u64)> {
        let pchain = get_chain_mut!(self, &pcid);
        // Lookup in block array.
        let (i, key_next) = pchain.find_blockref(key_end + 1, key_beg)?;
        // Lookup in chain.
        let base = pchain.as_blockref()?;
        if let Some((x, key_next)) = pchain.find_child(key_next, key_beg, key_end)? {
            if i == base.len() {
                // Only chain matched.
                if x.key > key_end {
                    // If the bref is out of bounds we've exhausted our search.
                    Ok((crate::chain::CID_NONE, usize::MAX, key_next))
                } else {
                    Ok((x.cid, usize::MAX, key_next))
                }
            } else {
                // Both in-memory and blockref matched, select the nearer element.
                //
                // If both are flush with the left-hand side or both are the
                // same distance away, select the chain.  In this situation the
                // chain must have been loaded from the matching blockmap.
                if (x.key <= key_beg && base[i].key <= key_beg) || x.key == base[i].key {
                    assert_eq!(x.key, base[i].key, "{:x} vs {:x}", x.key, base[i].key);
                    if x.key > key_end {
                        // If the bref is out of bounds we've exhausted our search.
                        Ok((crate::chain::CID_NONE, usize::MAX, key_next))
                    } else {
                        Ok((x.cid, usize::MAX, key_next))
                    }
                } else {
                    // Select the nearer key.
                    if x.key < base[i].key {
                        Ok((x.cid, usize::MAX, key_next))
                    } else {
                        Ok((crate::chain::CID_NONE, i, key_next))
                    }
                }
            }
        } else if true {
            if i == base.len() {
                // Neither matched.
                Ok((crate::chain::CID_NONE, usize::MAX, key_next))
            } else {
                // Only blockref matched.
                if base[i].key > key_end {
                    // If the bref is out of bounds we've exhausted our search.
                    Ok((crate::chain::CID_NONE, usize::MAX, key_next))
                } else {
                    Ok((crate::chain::CID_NONE, i, key_next))
                }
            }
        } else {
            unreachable!();
        }
    }

    /// # Errors
    pub fn dump_inode_chain(&self, ip: &crate::inode::Inode) -> crate::Result<()> {
        self.dump_chain(ip.cid)
    }

    /// # Errors
    pub fn dump_chain(&self, cid: crate::chain::Cid) -> crate::Result<()> {
        if std::env::var(HAMMER2_COMPAT).is_ok() {
            Ok(self.dump_chain_impl_compat(cid, 0, 0, -1, 'i')?)
        } else {
            Ok(self.dump_chain_impl(cid, 0, 0, -1)?)
        }
    }

    fn dump_vchain(&self) -> nix::Result<()> {
        if std::env::var(HAMMER2_COMPAT).is_ok() {
            self.dump_chain_impl_compat(crate::chain::CID_VCHAIN, 0, 0, -1, 'v')
        } else {
            self.dump_chain_impl(crate::chain::CID_VCHAIN, 0, 0, -1)
        }
    }

    fn dump_fchain(&self) -> nix::Result<()> {
        if std::env::var(HAMMER2_COMPAT).is_ok() {
            self.dump_chain_impl_compat(crate::chain::CID_FCHAIN, 0, 0, -1, 'f')
        } else {
            self.dump_chain_impl(crate::chain::CID_FCHAIN, 0, 0, -1)
        }
    }

    fn dump_chain_impl_compat(
        &self,
        cid: crate::chain::Cid,
        tab: usize,
        bi: usize,
        depth: isize,
        pfx: char,
    ) -> nix::Result<()> {
        assert!(depth == -1 || depth >= 0);
        let mut depth = depth;
        if depth != -1 {
            if depth == 0 {
                return Ok(());
            }
            depth -= 1;
        }
        let chain = get_chain!(self, &cid);
        let filename = if chain.bref.typ == crate::fs::HAMMER2_BREF_TYPE_INODE && chain.has_data() {
            Some(
                chain
                    .as_inode_data()
                    .get_filename_string()
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        let indent = " ".repeat(tab);
        println!("{indent}{pfx}-chain {bi} {}", chain.bref);
        print!(
            "{indent}      ({}) flags {:08x}",
            if let Some(ref filename) = filename {
                filename
            } else {
                "?"
            },
            chain.get_flags(),
        );
        let v = chain.get_child();
        if v.is_empty() {
            println!();
        } else {
            println!(" {{");
            for (i, &cid) in v.iter().enumerate() {
                self.dump_chain_impl_compat(cid, tab + 4, i, depth, 'a')?;
            }
            if let Some(filename) = filename {
                println!("{indent}}}({filename})");
            } else {
                println!("{indent}}}");
            }
        }
        Ok(())
    }

    fn dump_chain_impl(
        &self,
        cid: crate::chain::Cid,
        tab: usize,
        bi: usize,
        depth: isize,
    ) -> nix::Result<()> {
        assert!(depth == -1 || depth >= 0);
        let mut depth = depth;
        if depth != -1 {
            if depth == 0 {
                return Ok(());
            }
            depth -= 1;
        }
        let chain = get_chain!(self, &cid);
        let indent = " ".repeat(tab);
        println!(
            "{indent}{} #{} [{}] \"{}\" {:08x} {}/{}",
            match cid {
                crate::chain::CID_NONE => {
                    log::error!("bad cid {cid}");
                    return Err(nix::errno::Errno::EINVAL);
                }
                crate::chain::CID_VCHAIN => "v-chain".to_string(),
                crate::chain::CID_FCHAIN => "f-chain".to_string(),
                _ => format!("chain[{bi}]"),
            },
            chain.cid,
            chain.bref,
            chain.get_name().unwrap_or_default(),
            chain.get_flags(),
            chain.get_data_size(),
            chain.get_udata_size(),
        );
        if chain.has_data() {
            for (i, bref) in chain.as_blockref_safe().iter().enumerate() {
                if bref.typ != crate::fs::HAMMER2_BREF_TYPE_EMPTY {
                    println!("{indent}  bref[{i}] [{bref}]");
                }
            }
        } else {
            println!("{indent}  (no chain data)");
        }
        for (i, &cid) in chain.get_child().iter().enumerate() {
            self.dump_chain_impl(cid, tab + 2, i, depth)?;
        }
        Ok(())
    }

    fn add_inode(&mut self, ip: crate::inode::Inode) -> nix::Result<()> {
        if let Some(ip) = self.nmap.get(&ip.meta.inum) {
            log::error!("collision {ip:?}");
            return Err(nix::errno::Errno::EEXIST);
        }
        let inum = ip.meta.inum;
        assert!(self.nmap.insert(inum, ip).is_none());
        Ok(())
    }

    fn remove_inode(&mut self, inum: u64) -> nix::Result<crate::inode::Inode> {
        match self.nmap.remove(&inum) {
            Some(ip) => Ok(ip),
            None => Err(nix::errno::Errno::ENOENT),
        }
    }

    fn set_inode(&mut self, inum: u64) -> nix::Result<bool> {
        if self.nmap.contains_key(&inum) {
            let ip = get_inode!(self, &inum);
            assert_eq!(ip.meta.inum, inum);
            if ip.cid == crate::chain::CID_NONE {
                return Err(nix::errno::Errno::EINVAL);
            }
            Ok(true) // already exists
        } else {
            let mut ip = crate::inode::Inode::new_empty();
            ip.meta.inum = inum;
            self.add_inode(ip)?;
            Ok(false)
        }
    }

    fn set_inode_from_xop(&mut self, head: &crate::xop::XopHeader) -> nix::Result<(u64, bool)> {
        let chain = get_chain!(self, &head.collect()?);
        assert_eq!(chain.bref.typ, crate::fs::HAMMER2_BREF_TYPE_INODE);
        let ipdata = chain.as_inode_data();
        let inum = ipdata.meta.inum;
        if self.nmap.contains_key(&inum) {
            let ip = get_inode_mut!(self, &inum);
            assert_eq!(ip.meta.inum, inum);
            if ip.cid == crate::chain::CID_NONE || ip.cid != chain.cid {
                return Err(nix::errno::Errno::EINVAL);
            }
            Ok((inum, true)) // already exists
        } else {
            self.add_inode(crate::inode::Inode::new(&ipdata.meta, chain.cid))?;
            Ok((inum, false))
        }
    }

    /// # Errors
    pub fn get_inode_chain(&mut self, inum: u64, how: u32) -> crate::Result<crate::chain::Cid> {
        let cid = get_inode!(self, &inum).cid;
        if cid != crate::chain::CID_NONE {
            self.load_chain(cid, how)?;
        }
        Ok(cid)
    }

    fn get_inode_chain_and_parent(
        &mut self,
        inum: u64,
        how: u32,
    ) -> crate::Result<(crate::chain::Cid, crate::chain::Cid)> {
        let cid = get_inode!(self, &inum).cid;
        if cid != crate::chain::CID_NONE {
            self.load_chain(cid, how)?;
        }
        let pcid = get_chain!(self, &cid).pcid;
        if pcid != crate::chain::CID_NONE {
            self.load_chain(pcid, how)?;
        }
        Ok((pcid, cid))
    }

    fn find_inode_chain(
        &mut self,
        inum: u64,
    ) -> crate::Result<(crate::chain::Cid, crate::chain::Cid)> {
        if self.nmap.contains_key(&inum) {
            let (pcid, cid) = self.get_inode_chain_and_parent(inum, 0)?;
            if cid != crate::chain::CID_NONE {
                return Ok((pcid, cid));
            }
        }
        let pcid = self.get_inode_chain(crate::inode::INUM_PFS_ROOT, 0)?;
        if pcid == crate::chain::CID_NONE {
            return Err(nix::errno::Errno::EIO.into());
        }
        let (pcid, cid, _) = self.lookup_chain(pcid, inum, inum, 0)?;
        if cid != crate::chain::CID_NONE {
            let chain = get_chain!(self, &cid);
            if chain.has_data() {
                let ipdata = chain.as_inode_data();
                if inum != ipdata.meta.inum {
                    log::error!(
                        "lookup inum {inum:016x}, got inum {:016x}",
                        ipdata.meta.inum
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
        }
        Ok((pcid, cid))
    }

    #[must_use]
    pub fn get_inode_embed_stats(&self, inum: u64) -> &crate::fs::Hammer2BlockrefEmbedStats {
        get_chain!(self, &get_inode!(self, &inum).cid)
            .bref
            .embed_as::<crate::fs::Hammer2BlockrefEmbedStats>()
    }

    fn xop_nresolve(&mut self, arg: &mut crate::xop::XopNresolve) -> crate::Result<()> {
        let pcid = self.get_inode_chain(arg.head.inum1, RESOLVE_ALWAYS)?;
        if pcid == crate::chain::CID_NONE {
            return Err(nix::errno::Errno::EIO.into());
        }
        let lhc = crate::subs::dirhash(arg.head.name1.as_bytes());
        let (mut pcid, mut cid, _) = self.lookup_chain(
            pcid,
            lhc,
            lhc + crate::fs::HAMMER2_DIRHASH_LOMASK,
            LOOKUP_ALWAYS,
        )?;
        while cid != crate::chain::CID_NONE {
            let chain = get_chain!(self, &cid);
            if chain.match_name(&arg.head.name1) {
                break;
            }
            (pcid, cid, _) = self.get_next_chain(
                pcid,
                cid,
                lhc + crate::fs::HAMMER2_DIRHASH_LOMASK,
                LOOKUP_ALWAYS,
            )?;
        }
        if cid != crate::chain::CID_NONE {
            let chain = get_chain!(self, &cid);
            if chain.bref.typ == crate::fs::HAMMER2_BREF_TYPE_DIRENT {
                let lhc = chain.bref.embed_as::<crate::fs::Hammer2DirentHead>().inum;
                (_, cid) = self.find_inode_chain(lhc)?;
            }
        }
        arg.head.feed(cid);
        Ok(())
    }

    fn xop_readdir(&mut self, arg: &mut crate::xop::XopReaddir) -> crate::Result<()> {
        let pcid = self.get_inode_chain(arg.head.inum1, RESOLVE_ALWAYS)?;
        if pcid == crate::chain::CID_NONE {
            return Err(nix::errno::Errno::EIO.into());
        }
        let (mut pcid, mut cid, _) = self.lookup_chain(pcid, arg.lkey, arg.lkey, 0)?;
        if cid == crate::chain::CID_NONE {
            (pcid, cid, _) = self.lookup_chain(pcid, arg.lkey, crate::fs::HAMMER2_KEY_MAX, 0)?;
        }
        while cid != crate::chain::CID_NONE {
            arg.head.feed(cid);
            (pcid, cid, _) = self.get_next_chain(pcid, cid, crate::fs::HAMMER2_KEY_MAX, 0)?;
        }
        Ok(())
    }

    fn xop_bmap(&mut self, arg: &mut crate::xop::XopBmap) -> crate::Result<()> {
        let lbase = arg.lbn * crate::fs::HAMMER2_PBUFSIZE;
        assert_eq!(lbase & crate::fs::HAMMER2_PBUFMASK, 0);
        let pcid = self.get_inode_chain(arg.head.inum1, RESOLVE_ALWAYS)?;
        if pcid == crate::chain::CID_NONE {
            return Err(nix::errno::Errno::EIO.into());
        }
        // CID_NONE isn't necessarily an error.
        // It could be a zero filled data without physical block assigned.
        arg.offset = crate::fs::HAMMER2_OFF_MASK;
        let (_, cid, _) = self.lookup_chain(pcid, lbase, lbase, LOOKUP_ALWAYS)?;
        if cid == crate::chain::CID_NONE {
            return Err(nix::errno::Errno::ENOENT.into());
        }
        arg.offset = get_chain!(self, &cid).bref.get_raw_data_off();
        arg.head.feed(cid);
        Ok(())
    }

    fn xop_read(&mut self, arg: &mut crate::xop::XopRead) -> crate::Result<Vec<u8>> {
        let pcid = self.get_inode_chain(arg.head.inum1, RESOLVE_ALWAYS)?;
        if pcid == crate::chain::CID_NONE {
            return Err(nix::errno::Errno::EIO.into());
        }
        let (_, cid, _) = self.lookup_chain(pcid, arg.lbase, arg.lbase, LOOKUP_ALWAYS)?;
        if cid == crate::chain::CID_NONE {
            return Ok(vec![0; crate::fs::HAMMER2_PBUFSIZE.try_into().unwrap()]);
        }
        arg.head.feed(cid);
        let chain = get_chain_mut!(self, &arg.head.collect()?);
        Ok(if self.opt.nodatacache {
            chain.read_data()
        } else {
            chain.read_cache_data()
        }?)
    }

    /// # Errors
    pub fn nresolve_path(&mut self, path: &str) -> crate::Result<u64> {
        if path.is_empty() {
            return Err(nix::errno::Errno::EINVAL.into());
        }
        // Only allow mounted PFS to avoid inum collision.
        let mut inum = crate::inode::INUM_PFS_ROOT;
        for cnp in &libfs::fs::split_path(path) {
            inum = self.nresolve(inum, cnp)?;
        }
        Ok(inum)
    }

    /// # Errors
    pub fn nresolve(&mut self, dinum: u64, cnp: &str) -> crate::Result<u64> {
        // Only allow mounted PFS to avoid inum collision.
        if dinum == crate::inode::INUM_SUP_ROOT {
            return Err(nix::errno::Errno::EINVAL.into());
        }
        match cnp {
            "." => Ok(dinum),
            ".." => Ok(get_inode!(self, &dinum).meta.iparent),
            _ => {
                let mut arg = crate::xop::XopNresolve::new(dinum, cnp);
                self.xop_nresolve(&mut arg)?;
                let (inum, _) = self.set_inode_from_xop(&arg.head)?;
                Ok(inum)
            }
        }
    }

    /// # Errors
    pub fn readdir(&mut self, dinum: u64) -> crate::Result<Vec<Dirent>> {
        let ip = get_inode!(self, &dinum);
        if ip.meta.typ != crate::fs::HAMMER2_OBJTYPE_DIRECTORY {
            return Err(nix::errno::Errno::ENOTDIR.into());
        }
        let mut v = vec![
            Dirent::new(ip.meta.inum, ip.meta.typ, "."),
            Dirent::new(
                ip.meta.iparent & crate::fs::HAMMER2_DIRHASH_USERMSK,
                crate::fs::HAMMER2_OBJTYPE_DIRECTORY,
                "..",
            ),
        ];
        let mut arg = crate::xop::XopReaddir::new(dinum, 2 | crate::fs::HAMMER2_DIRHASH_VISIBLE);
        self.xop_readdir(&mut arg)?;
        let dirents = match arg.head.collect_all() {
            Ok(v) => v,
            Err(nix::errno::Errno::ENOENT) => vec![],
            Err(e) => return Err(e.into()),
        };
        for cid in &dirents {
            let chain = get_chain!(self, &cid);
            match chain.bref.typ {
                crate::fs::HAMMER2_BREF_TYPE_INODE => {
                    let ipdata = chain.as_inode_data();
                    if let Some(s) = chain.get_name() {
                        v.push(Dirent::new(
                            ipdata.meta.inum & crate::fs::HAMMER2_DIRHASH_USERMSK,
                            ipdata.meta.typ,
                            &s,
                        ));
                    } else {
                        return Err(nix::errno::Errno::EINVAL.into());
                    }
                }
                crate::fs::HAMMER2_BREF_TYPE_DIRENT => {
                    let dirent = chain.bref.embed_as::<crate::fs::Hammer2DirentHead>();
                    if let Some(s) = chain.get_name() {
                        v.push(Dirent::new(dirent.inum, dirent.typ, &s));
                    } else {
                        return Err(nix::errno::Errno::EINVAL.into());
                    }
                }
                _ => {
                    log::error!("bad blockref type {}", chain.bref.typ);
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
        }
        Ok(v)
    }

    /// # Errors
    /// # Panics
    pub fn bmap(&mut self, inum: u64, lbn: u64) -> crate::Result<u64> {
        let mut arg = crate::xop::XopBmap::new(inum, lbn);
        self.xop_bmap(&mut arg)?;
        if arg.offset == crate::fs::HAMMER2_OFF_MASK {
            return Err(nix::errno::Errno::EINVAL.into());
        }
        assert_eq!(crate::extra::conv_offset_to_radix(arg.offset), 0);
        match self.fso.get_volume_mut(arg.offset) {
            Some(vol) => Ok(arg.offset - vol.get_offset() / libfs::os::DEV_BSIZE),
            None => Err(nix::errno::Errno::ENODEV.into()),
        }
    }

    /// # Errors
    pub fn readlink(&mut self, inum: u64, buf: &mut [u8]) -> crate::Result<u64> {
        let ip = get_inode!(self, &inum);
        if ip.meta.typ != crate::fs::HAMMER2_OBJTYPE_SOFTLINK {
            return Err(nix::errno::Errno::EINVAL.into());
        }
        self.pread_impl(inum, buf, 0)
    }

    /// # Errors
    pub fn pread(&mut self, inum: u64, buf: &mut [u8], offset: u64) -> crate::Result<u64> {
        let ip = get_inode!(self, &inum);
        if ip.meta.typ == crate::fs::HAMMER2_OBJTYPE_DIRECTORY {
            return Err(nix::errno::Errno::EISDIR.into());
        }
        if ip.meta.typ != crate::fs::HAMMER2_OBJTYPE_REGFILE {
            return Err(nix::errno::Errno::EINVAL.into());
        }
        self.pread_impl(inum, buf, offset)
    }

    fn pread_impl(&mut self, inum: u64, buf: &mut [u8], offset: u64) -> crate::Result<u64> {
        let mut buf = buf;
        let mut resid = buf.len().try_into().unwrap();
        let start_offset = offset;
        let mut offset = offset;
        let mut total = 0;
        let ipsize = get_inode!(self, &inum).meta.size;

        while resid > 0 && offset < ipsize {
            let lbase = offset & !crate::fs::HAMMER2_PBUFMASK;
            let mut arg = crate::xop::XopRead::new(inum, lbase);
            let b = self.xop_read(&mut arg)?;
            assert!(b.len() <= crate::fs::HAMMER2_PBUFSIZE.try_into().unwrap());
            let loff = offset - lbase;
            let mut n = crate::fs::HAMMER2_PBUFSIZE - loff;
            if n > resid {
                n = resid;
            }
            if n > ipsize - offset {
                n = ipsize - offset;
            }
            let i = loff.try_into().unwrap();
            let x = n.try_into().unwrap();
            buf[..x].copy_from_slice(&b[i..i + x]);
            buf = &mut buf[x..];
            total += n;
            offset += n;
            resid -= n;
        }
        assert!(total <= ipsize - start_offset);
        Ok(total)
    }

    fn init_vchain(&mut self) -> nix::Result<()> {
        let mut bref = crate::fs::Hammer2Blockref::new(crate::fs::HAMMER2_BREF_TYPE_VOLUME);
        bref.data_off = crate::fs::HAMMER2_PBUFRADIX.try_into().unwrap();
        bref.mirror_tid = self.voldata.mirror_tid;
        bref.modify_tid = bref.mirror_tid;

        let mut chain = crate::chain::Chain::new(&bref, crate::chain::CID_VCHAIN)?;
        chain.set_data(libfs::cast::as_u8_slice(&self.voldata).to_vec());
        assert!(!self.cmap.contains_key(&chain.cid));
        assert!(self.cmap.insert(chain.cid, chain).is_none());
        assert!(self.cmap.contains_key(&crate::chain::CID_VCHAIN));
        Ok(())
    }

    fn remove_vchain(&mut self) -> nix::Result<()> {
        if self.cmap.remove(&crate::chain::CID_VCHAIN).is_some() {
            Ok(())
        } else {
            Err(nix::errno::Errno::ENOENT)
        }
    }

    fn init_fchain(&mut self) -> nix::Result<()> {
        let mut bref = crate::fs::Hammer2Blockref::new(crate::fs::HAMMER2_BREF_TYPE_FREEMAP);
        bref.data_off = crate::fs::HAMMER2_PBUFRADIX.try_into().unwrap();
        bref.mirror_tid = self.voldata.mirror_tid;
        bref.modify_tid = bref.mirror_tid;
        bref.methods = crate::fs::enc_check(crate::fs::HAMMER2_CHECK_FREEMAP)
            | crate::fs::enc_comp(crate::fs::HAMMER2_COMP_NONE);

        let mut chain = crate::chain::Chain::new(&bref, crate::chain::CID_FCHAIN)?;
        chain.set_data(libfs::cast::as_u8_slice(&self.voldata).to_vec());
        assert!(!self.cmap.contains_key(&chain.cid));
        assert!(self.cmap.insert(chain.cid, chain).is_none());
        assert!(self.cmap.contains_key(&crate::chain::CID_FCHAIN));
        Ok(())
    }

    fn remove_fchain(&mut self) -> nix::Result<()> {
        if self.cmap.remove(&crate::chain::CID_FCHAIN).is_some() {
            Ok(())
        } else {
            Err(nix::errno::Errno::ENOENT)
        }
    }

    fn init_sup_root_inode(&mut self, cid: crate::chain::Cid) -> nix::Result<()> {
        let chain = get_chain!(self, &cid);
        log::debug!("{}", chain.as_inode_data());
        let (inum, exists) =
            self.set_inode_from_xop(&crate::xop::XopHeader::dummy_new(chain.cid))?;
        assert_eq!(inum, crate::inode::INUM_SUP_ROOT);
        assert!(!exists);
        assert!(self.nmap.contains_key(&crate::inode::INUM_SUP_ROOT));
        Ok(())
    }

    fn init_pfs_root_inode(&mut self, cid: crate::chain::Cid) -> nix::Result<()> {
        let chain = get_chain!(self, &cid);
        let ipdata = chain.as_inode_data();
        let meta = ipdata.meta;
        log::debug!("{ipdata}");
        assert!(!self.set_inode(ipdata.meta.inum)?);
        assert!(self.nmap.contains_key(&crate::inode::INUM_PFS_ROOT));
        let ip = get_inode_mut!(self, &crate::inode::INUM_PFS_ROOT);
        ip.meta = meta;
        ip.cid = cid;
        Ok(())
    }

    /// # Errors
    /// # Panics
    #[allow(clippy::too_many_lines)]
    pub fn mount(spec: &str, args: &[&str]) -> crate::Result<Self> {
        log::debug!("{spec} {args:?}");
        // Allocate option.
        let opt = crate::option::Opt::new(args)?;
        log::debug!("{opt:?}");
        // Parse label.
        let (spec, label) = if let Some(i) = spec.find('@') {
            if i == spec.len() - 1 {
                (&spec[..i], crate::inode::DEFAULT_PFS_LABEL)
            } else {
                (&spec[..i], &spec[i + 1..])
            }
        } else {
            (spec, crate::inode::DEFAULT_PFS_LABEL)
        };
        log::debug!("spec \"{spec}\" label \"{label}\"");
        if spec.is_empty() {
            log::error!("empty spec");
            return Err(nix::errno::Errno::EINVAL.into());
        }
        assert!(!label.is_empty());
        // Allocate ondisk.
        let fso = crate::ondisk::init_quiet(spec, true)?;
        log::debug!("{fso:?}");

        // Allocate PFS.
        let mut pmp = Self::new(fso, opt)?;
        if !pmp.voldata.is_hbo() {
            log::error!("reverse-endian not supported");
            return Err(nix::errno::Errno::EINVAL.into());
        }
        pmp.imap.max = match pmp.opt.cidalloc {
            crate::option::CidAllocMode::Linear => crate::chain::Cid::MAX - 1,
            crate::option::CidAllocMode::Bitmap => {
                let x = 4usize << 20; // 512KB
                let n = x.div_ceil(libfs::bitmap::BLOCK_BITS) * libfs::bitmap::BLOCK_BITS;
                log::debug!("imap: {} bits, {} bytes", n, n / 8);
                pmp.imap.chunk = libfs::bitmap::Bitmap::new(n)?;
                (n - 1).try_into().unwrap()
            }
        };
        pmp.init_vchain()?;
        assert_eq!(pmp.cmap.len(), 1);
        pmp.init_fchain()?;
        assert_eq!(pmp.cmap.len(), 2);

        // First locate the super-root inode, which is key 0
        // relative to the volume header's blockset.
        //
        // Then locate the root inode by scanning the directory keyspace
        // represented by the label.
        let chain = get_chain!(pmp, &crate::chain::CID_VCHAIN);
        let cid = chain.cid;
        pmp.load_chain(cid, RESOLVE_ALWAYS)?;
        let (_, cid, _) = pmp.lookup_chain(
            cid,
            crate::fs::HAMMER2_SROOT_KEY,
            crate::fs::HAMMER2_SROOT_KEY,
            0,
        )?;
        if cid == crate::chain::CID_NONE {
            log::error!("super-root not found");
            return Err(nix::errno::Errno::EINVAL.into());
        }
        pmp.init_sup_root_inode(cid)?;
        assert_eq!(pmp.nmap.len(), 1);

        // Scan PFSs under the super-root.
        let pcid = pmp.get_inode_chain(crate::inode::INUM_SUP_ROOT, RESOLVE_ALWAYS)?;
        let (mut pcid, mut cid, _) = pmp.lookup_chain(
            pcid,
            crate::fs::HAMMER2_KEY_MIN,
            crate::fs::HAMMER2_KEY_MAX,
            0,
        )?;
        while cid != crate::chain::CID_NONE {
            let chain = get_chain!(pmp, &cid);
            if chain.bref.typ != crate::fs::HAMMER2_BREF_TYPE_INODE {
                log::error!("non inode chain under super-root: {}", chain.bref);
                return Err(nix::errno::Errno::EINVAL.into());
            }
            log::debug!("{}", chain.as_inode_data());
            (pcid, cid, _) = pmp.get_next_chain(pcid, cid, crate::fs::HAMMER2_KEY_MAX, 0)?;
        }

        // Lookup the mount point under the media-localized super-root.
        let pcid = pmp.get_inode_chain(crate::inode::INUM_SUP_ROOT, RESOLVE_ALWAYS)?;
        let lhc = crate::subs::dirhash(label.as_bytes());
        let (mut pcid, mut cid, _) =
            pmp.lookup_chain(pcid, lhc, lhc + crate::fs::HAMMER2_DIRHASH_LOMASK, 0)?;
        while cid != crate::chain::CID_NONE {
            let chain = get_chain!(pmp, &cid);
            if chain.bref.typ == crate::fs::HAMMER2_BREF_TYPE_INODE {
                match chain.as_inode_data().get_filename_string() {
                    Ok(s) => {
                        if s == label {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("{e}");
                        return Err(nix::errno::Errno::EINVAL.into());
                    }
                }
            }
            (pcid, cid, _) =
                pmp.get_next_chain(pcid, cid, lhc + crate::fs::HAMMER2_DIRHASH_LOMASK, 0)?;
        }
        if cid == crate::chain::CID_NONE {
            log::error!("PFS label \"{label}\" not found");
            return Err(nix::errno::Errno::ENOENT.into());
        }
        pmp.init_pfs_root_inode(cid)?;
        assert_eq!(pmp.nmap.len(), 2);
        pmp.label = label.to_string();
        assert!(!pmp.label.is_empty());

        Ok(pmp)
    }

    /// # Errors
    /// # Panics
    pub fn unmount(&mut self) -> crate::Result<()> {
        assert!(self.cmap.contains_key(&crate::chain::CID_VCHAIN));
        assert!(self.cmap.contains_key(&crate::chain::CID_FCHAIN));
        assert!(self.nmap.contains_key(&crate::inode::INUM_SUP_ROOT));
        assert!(self.nmap.contains_key(&crate::inode::INUM_PFS_ROOT));
        assert!(!self.nmap.is_empty());
        self.clear_chain()?;
        assert!(self.cmap.contains_key(&crate::chain::CID_VCHAIN));
        assert!(self.cmap.contains_key(&crate::chain::CID_FCHAIN));
        assert!(self.nmap.is_empty());
        self.dump_vchain()?;
        self.dump_fchain()?;
        assert_eq!(self.cmap.len(), 2);
        self.remove_vchain()?;
        assert_eq!(self.cmap.len(), 1);
        self.remove_fchain()?;
        assert!(self.cmap.is_empty());
        match self.opt.cidalloc {
            crate::option::CidAllocMode::Linear => {
                assert!(self.imap.pool.is_empty());
                assert!(self.imap.chunk.is_empty());
            }
            crate::option::CidAllocMode::Bitmap => {
                log::debug!("imap: {} pool entries", self.imap.pool.len());
            }
        }
        Ok(())
    }

    /// # Errors
    /// # Panics
    pub fn stat(&self, inum: u64) -> crate::Result<Stat> {
        let Some(ip) = self.nmap.get(&inum) else {
            return Err(nix::errno::Errno::ENOENT.into());
        };
        let mode = match ip.meta.typ {
            crate::fs::HAMMER2_OBJTYPE_DIRECTORY => libc::S_IFDIR,
            crate::fs::HAMMER2_OBJTYPE_REGFILE => libc::S_IFREG,
            crate::fs::HAMMER2_OBJTYPE_FIFO => libc::S_IFIFO,
            crate::fs::HAMMER2_OBJTYPE_CDEV => libc::S_IFCHR,
            crate::fs::HAMMER2_OBJTYPE_BDEV => libc::S_IFBLK,
            crate::fs::HAMMER2_OBJTYPE_SOFTLINK => libc::S_IFLNK,
            crate::fs::HAMMER2_OBJTYPE_SOCKET => libc::S_IFSOCK,
            _ => 0,
        };
        Ok(Stat {
            st_dev: 0,
            st_ino: ip.meta.inum,
            st_nlink: ip.meta.nlinks.try_into().unwrap(),
            st_mode: StatMode::try_from(ip.meta.mode).unwrap() | mode,
            st_uid: crate::subs::conv_uuid_to_unix_xid_from_bytes(&ip.meta.uid),
            st_gid: crate::subs::conv_uuid_to_unix_xid_from_bytes(&ip.meta.gid),
            st_rdev: 0,
            st_size: ip.meta.size,
            st_blksize: crate::fs::HAMMER2_PBUFSIZE.try_into().unwrap(),
            st_blocks: if ip.meta.typ == crate::fs::HAMMER2_OBJTYPE_DIRECTORY {
                crate::fs::HAMMER2_INODE_BYTES
            } else {
                ip.meta.size
            } / libfs::os::DEV_BSIZE,
            st_atime: crate::subs::conv_time_to_timespec(ip.meta.atime),
            st_mtime: crate::subs::conv_time_to_timespec(ip.meta.mtime),
            st_ctime: crate::subs::conv_time_to_timespec(ip.meta.ctime),
        })
    }

    /// # Errors
    /// # Panics
    pub fn statfs(&mut self) -> crate::Result<StatFs> {
        let cid = self.get_inode_chain(crate::inode::INUM_PFS_ROOT, RESOLVE_MAYBE)?;
        let bsize = crate::fs::HAMMER2_PBUFSIZE;
        Ok(StatFs {
            f_bsize: bsize.try_into().unwrap(),
            f_blocks: self.voldata.allocator_size / bsize,
            f_bfree: self.voldata.allocator_free / bsize,
            f_bavail: self.voldata.allocator_free / bsize,
            f_files: get_chain!(self, &cid)
                .bref
                .embed_as::<crate::fs::Hammer2BlockrefEmbedStats>()
                .inode_count,
            f_ffree: 0,
            f_namelen: 0,
            f_frsize: bsize.try_into().unwrap(),
        })
    }
}

#[cfg(test)]
mod tests {
    const HAMMER2_NODATACACHE: &str = "HAMMER2_NODATACACHE"; // option
    const HAMMER2_DEBUG: &str = "HAMMER2_DEBUG"; // option
    const HAMMER2_DEVICE: &str = "HAMMER2_DEVICE";
    const HAMMER2_PATH: &str = "HAMMER2_PATH";

    fn init_std_logger() -> Result<(), log::SetLoggerError> {
        let env = env_logger::Env::default().filter_or("RUST_LOG", "trace");
        env_logger::try_init_from_env(env)
    }

    fn read_all(pmp: &mut super::Hammer2, inum: u64) -> crate::Result<Vec<u8>> {
        let st = pmp.stat(inum)?;
        let mut resid = st.st_size;
        let size = if resid / 10 > 0 { resid / 10 } else { resid };
        let mut offset = 0;
        let mut v = vec![];
        while resid > 0 {
            let b = pmp.preadx(inum, size, offset)?;
            let n = u64::try_from(b.len()).unwrap();
            v.extend(b);
            offset += n;
            resid -= n;
        }
        assert_eq!(pmp.preadx(inum, size, offset)?.len(), 0);
        Ok(v)
    }

    fn is_zero(v: &[u8]) -> bool {
        if v.is_empty() {
            return true;
        }
        for x in v {
            if *x != 0 {
                return false;
            }
        }
        true
    }

    fn test_hammer2_path(pmp: &mut super::Hammer2, f: &str) {
        log::info!("{f}");
        match pmp.nresolve_path(f) {
            Ok(inum) => {
                log::info!("{inum} {inum:#x}");
                match pmp.stat(inum) {
                    Ok(st) => {
                        log::info!("{st:?}");
                        match st.st_mode & libc::S_IFMT {
                            libc::S_IFDIR => match pmp.readdir(inum) {
                                Ok(v) => log::info!("{}: {v:?}", v.len()),
                                Err(e) => panic!("{e}"),
                            },
                            libc::S_IFREG => {
                                let (sum1, is_zero1) = match pmp.read_all(inum) {
                                    Ok(v) => {
                                        assert_eq!(v.len(), st.st_size.try_into().unwrap());
                                        match libfs::string::b2s(&v) {
                                            Ok(v) => println!("{v}"),
                                            Err(e) => panic!("{e}"),
                                        }
                                        (hex::encode(crate::sha::sha256(&v)), is_zero(&v))
                                    }
                                    Err(e) => panic!("{e}"),
                                };
                                log::info!("sha256: {sum1}");
                                let (sum2, is_zero2) = match read_all(pmp, inum) {
                                    Ok(v) => {
                                        assert_eq!(v.len(), st.st_size.try_into().unwrap());
                                        match libfs::string::b2s(&v) {
                                            Ok(v) => println!("{v}"),
                                            Err(e) => panic!("{e}"),
                                        }
                                        (hex::encode(crate::sha::sha256(&v)), is_zero(&v))
                                    }
                                    Err(e) => panic!("{e}"),
                                };
                                log::info!("sha256: {sum2}");
                                assert_eq!(sum1, sum2);
                                assert_eq!(is_zero1, is_zero2);
                                if !is_zero1 {
                                    match pmp.bmap(inum, 0) {
                                        Ok(v) => log::info!("{v:016x}"),
                                        Err(e) => panic!("{e}"),
                                    }
                                }
                            }
                            libc::S_IFLNK => match pmp.readlinkx(inum) {
                                Ok(v) => {
                                    log::info!("\"{v}\"");
                                    assert_eq!(v.len(), st.st_size.try_into().unwrap());
                                    if !is_zero(v.as_bytes()) {
                                        match pmp.bmap(inum, 0) {
                                            Ok(v) => log::info!("{v:016x}"),
                                            Err(e) => panic!("{e}"),
                                        }
                                    }
                                }
                                Err(e) => panic!("{e}"),
                            },
                            _ => (),
                        }
                    }
                    Err(e) => panic!("{e}"),
                }
            }
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn test_hammer2_mount() {
        if let Ok(spec) = std::env::var(HAMMER2_DEVICE) {
            let _ = init_std_logger();
            let mut args = vec![];
            if std::env::var(HAMMER2_NODATACACHE).is_ok() {
                args.push("--nodatacache");
            }
            if std::env::var(HAMMER2_DEBUG).is_ok() {
                args.push("--debug");
            }
            // mount
            let mut pmp = match super::Hammer2::mount(&spec, &args) {
                Ok(v) => v,
                Err(e) => panic!("{e}"),
            };
            // dump_chain
            if let Err(e) = pmp.dump_vchain() {
                panic!("{e}");
            }
            if let Err(e) = pmp.dump_fchain() {
                panic!("{e}");
            }
            // statfs
            match pmp.statfs() {
                Ok(v) => log::info!("{v:?}"),
                Err(e) => panic!("{e}"),
            }
            // stat
            match pmp.stat(crate::inode::INUM_SUP_ROOT) {
                Ok(v) => log::info!("{v:?}"),
                Err(e) => panic!("{e}"),
            }
            match pmp.stat(crate::inode::INUM_PFS_ROOT) {
                Ok(v) => log::info!("{v:?}"),
                Err(e) => panic!("{e}"),
            }
            // nresolve
            match pmp.nresolve(crate::inode::INUM_PFS_ROOT, ".") {
                Ok(v) => assert_eq!(v, crate::inode::INUM_PFS_ROOT),
                Err(e) => panic!("{e}"),
            }
            match pmp.nresolve(crate::inode::INUM_PFS_ROOT, "..") {
                Ok(v) => assert_eq!(v, crate::inode::INUM_SUP_ROOT),
                Err(e) => panic!("{e}"),
            }
            match pmp.nresolve_path("/") {
                Ok(v) => assert_eq!(v, crate::inode::INUM_PFS_ROOT),
                Err(e) => panic!("{e}"),
            }
            // readdir
            match pmp.readdir(crate::inode::INUM_SUP_ROOT) {
                Ok(v) => log::info!("{}: {v:?}", v.len()),
                Err(e) => panic!("{e}"),
            }
            match pmp.readdir(crate::inode::INUM_PFS_ROOT) {
                Ok(v) => log::info!("{}: {v:?}", v.len()),
                Err(e) => panic!("{e}"),
            }
            // bmap
            match pmp.bmap(crate::inode::INUM_SUP_ROOT, 0) {
                Ok(v) => log::info!("{v:016x}"),
                Err(crate::Error::Errno(nix::errno::Errno::ENOENT)) => (),
                Err(e) => panic!("{e}"),
            }
            match pmp.bmap(crate::inode::INUM_PFS_ROOT, 0) {
                Ok(v) => log::info!("{v:016x}"),
                Err(crate::Error::Errno(nix::errno::Errno::ENOENT)) => (),
                Err(e) => panic!("{e}"),
            }
            // readlink
            match pmp.readlinkx(crate::inode::INUM_SUP_ROOT) {
                Ok(v) => panic!("{v:?}"),
                Err(crate::Error::Errno(nix::errno::Errno::EINVAL)) => (),
                Err(e) => panic!("{e}"),
            }
            match pmp.readlinkx(crate::inode::INUM_PFS_ROOT) {
                Ok(v) => panic!("{v:?}"),
                Err(crate::Error::Errno(nix::errno::Errno::EINVAL)) => (),
                Err(e) => panic!("{e}"),
            }
            // read
            match pmp.preadx(crate::inode::INUM_SUP_ROOT, 1, 0) {
                Ok(v) => panic!("{v:?}"),
                Err(crate::Error::Errno(nix::errno::Errno::EISDIR)) => (),
                Err(e) => panic!("{e}"),
            }
            match pmp.preadx(crate::inode::INUM_PFS_ROOT, 1, 0) {
                Ok(v) => panic!("{v:?}"),
                Err(crate::Error::Errno(nix::errno::Errno::EISDIR)) => (),
                Err(e) => panic!("{e}"),
            }
            // env path
            if let Ok(f) = std::env::var(HAMMER2_PATH) {
                test_hammer2_path(&mut pmp, &f);
            }
            // dump_chain
            if let Err(e) = pmp.dump_vchain() {
                panic!("{e}");
            }
            if let Err(e) = pmp.dump_fchain() {
                panic!("{e}");
            }
            // unmount
            if let Err(e) = pmp.unmount() {
                panic!("{e}");
            }
        }
    }
}
