pub type Cid = u64;

pub const CID_NONE: Cid = 0;
pub(crate) const CID_VCHAIN: Cid = 1;
pub(crate) const CID_FCHAIN: Cid = 2;
pub(crate) const CID_CHAIN_OFFSET: Cid = 3;

const CHAIN_TESTED_GOOD: u32 = 0x0000_0100; // crc tested good
const CHAIN_COUNTED_BLOCKREF: u32 = 0x0000_2000; // block table stats
const CHAIN_ALL_MASK: u32 = CHAIN_TESTED_GOOD | CHAIN_COUNTED_BLOCKREF;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ChainKey {
    pub(crate) cid: Cid,
    pub(crate) key: u64,
    pub(crate) keybits: u8,
}

impl ChainKey {
    fn new(cid: Cid, key: u64, keybits: u8) -> Self {
        Self { cid, key, keybits }
    }

    fn new_from_chain(chain: &Chain) -> Self {
        Self::new(chain.cid, chain.bref.key, chain.bref.keybits)
    }
}

pub(crate) fn cmp_chain_key(x1: &ChainKey, x2: &ChainKey) -> std::cmp::Ordering {
    let x1_beg = x1.key;
    let x1_end = x1_beg + (1 << x1.keybits) - 1;
    let x2_beg = x2.key;
    let x2_end = x2_beg + (1 << x2.keybits) - 1;
    cmp_chain_key_impl(x1_beg, x1_end, x2_beg, x2_end)
}

fn cmp_chain_key_impl(x1_beg: u64, x1_end: u64, x2_beg: u64, x2_end: u64) -> std::cmp::Ordering {
    // Compare chains.  Overlaps are not supposed to happen and catch
    // any software issues early we count overlaps as a match.
    if x1_end < x2_beg {
        std::cmp::Ordering::Less // fully to the left
    } else if x1_beg > x2_end {
        std::cmp::Ordering::Greater // fully to the right
    } else {
        std::cmp::Ordering::Equal // overlap (must not cross edge boundary)
    }
}

#[derive(Debug, Default)]
pub struct Chain {
    pub(crate) bref: crate::fs::Hammer2Blockref,
    flags: u32,         // for CHAIN_xxx
    bytes: u64,         // physical data size
    live_zero: usize,   // blockref array opt
    live_count: usize,  // live (not deleted) chains
    cache_index: usize, // heur speeds up lookup
    data: Vec<u8>,
    udata: Vec<u8>,       // Rust
    pub(crate) cid: Cid,  // Rust
    pub(crate) pcid: Cid, // Rust
    ccids: Vec<ChainKey>, // Rust
}

impl Chain {
    pub(crate) fn new(bref: &crate::fs::Hammer2Blockref, cid: Cid) -> nix::Result<Self> {
        // Special case - radix of 0 indicates a chain that does not
        // need a data reference (context is completely embedded in the bref).
        let radix = bref.get_radix();
        let bytes = if radix != 0 { 1 << radix } else { 0 };
        match bref.typ {
            crate::fs::HAMMER2_BREF_TYPE_INODE
            | crate::fs::HAMMER2_BREF_TYPE_INDIRECT
            | crate::fs::HAMMER2_BREF_TYPE_DATA
            | crate::fs::HAMMER2_BREF_TYPE_DIRENT
            | crate::fs::HAMMER2_BREF_TYPE_FREEMAP_NODE
            | crate::fs::HAMMER2_BREF_TYPE_FREEMAP_LEAF
            | crate::fs::HAMMER2_BREF_TYPE_FREEMAP
            | crate::fs::HAMMER2_BREF_TYPE_VOLUME => Ok(Self {
                bref: *bref,
                bytes,
                cid,
                pcid: CID_NONE,
                ..Default::default()
            }),
            _ => Err(nix::errno::Errno::EINVAL),
        }
    }

    #[must_use]
    pub fn get_blockref(&self) -> &crate::fs::Hammer2Blockref {
        &self.bref
    }

    pub(crate) fn get_flags(&self) -> u32 {
        self.flags
    }

    pub(crate) fn set_flags(&mut self, flags: u32) {
        assert_eq!(flags & !CHAIN_ALL_MASK, 0);
        self.flags |= flags;
    }

    #[allow(dead_code)]
    pub(crate) fn clear_flags(&mut self, flags: u32) {
        assert_eq!(flags & !CHAIN_ALL_MASK, 0);
        self.flags &= !flags;
    }

    pub(crate) fn has_flags(&self, flags: u32) -> bool {
        assert_eq!(flags & !CHAIN_ALL_MASK, 0);
        (self.flags & flags) != 0
    }

    pub(crate) fn get_bytes(&self) -> u64 {
        self.bytes
    }

    #[must_use]
    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }

    pub(crate) fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub(crate) fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    pub(crate) fn has_udata(&self) -> bool {
        !self.udata.is_empty()
    }

    pub(crate) fn get_data_size(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn get_udata_size(&self) -> usize {
        self.udata.len()
    }

    pub(crate) fn get_first_child(&self) -> Option<Cid> {
        if self.has_child() {
            Some(self.ccids[0].cid)
        } else {
            None
        }
    }

    pub(crate) fn get_child(&self) -> Vec<Cid> {
        self.ccids.iter().map(|x| x.cid).collect()
    }

    pub(crate) fn add_child(&mut self, chain: &Chain) {
        self.ccids.push(ChainKey::new_from_chain(chain));
        self.ccids.sort_by(cmp_chain_key); // stable sort
    }

    pub(crate) fn remove_child(&mut self, cid: Cid) -> nix::Result<()> {
        if let Some(i) = self.ccids.iter().position(|x| x.cid == cid) {
            self.ccids.swap_remove(i);
            self.ccids.sort_by(cmp_chain_key); // stable sort
            Ok(())
        } else {
            Err(nix::errno::Errno::ENOENT)
        }
    }

    pub(crate) fn has_child(&self) -> bool {
        !self.ccids.is_empty()
    }

    #[allow(dead_code)]
    fn find_child_index(&self, x: &ChainKey) -> Option<usize> {
        let x_beg = x.key;
        let x_end = x_beg + (1 << x.keybits) - 1;
        self.find_child_index_impl(x_beg, x_end)
    }

    fn find_child_index_impl(&self, x1_beg: u64, x1_end: u64) -> Option<usize> {
        if !self.has_child() {
            return None;
        }
        let mut beg = 0;
        let mut end = self.ccids.len() - 1;
        let mut i = (beg + end) / 2;
        while beg <= end {
            let x2_beg = self.ccids[i].key;
            let x2_end = x2_beg + (1 << self.ccids[i].keybits) - 1;
            match cmp_chain_key_impl(x1_beg, x1_end, x2_beg, x2_end) {
                std::cmp::Ordering::Less => {
                    if i == 0 {
                        break;
                    }
                    end = i - 1;
                }
                std::cmp::Ordering::Greater => {
                    if i == usize::MAX {
                        break;
                    }
                    beg = i + 1;
                }
                std::cmp::Ordering::Equal => return Some(i), // first found
            }
            i = (beg + end) / 2;
        }
        None
    }

    #[allow(dead_code)]
    fn find_child_range(&self, x: &ChainKey) -> nix::Result<Option<(usize, usize)>> {
        let x_beg = x.key;
        let x_end = x_beg + (1 << x.keybits) - 1;
        self.find_child_range_impl(x_beg, x_end)
    }

    fn find_child_range_impl(
        &self,
        x1_beg: u64,
        x1_end: u64,
    ) -> nix::Result<Option<(usize, usize)>> {
        let Some(i) = self.find_child_index_impl(x1_beg, x1_end) else {
            return Ok(None);
        };
        let v = &self.ccids;
        let mut beg = i;
        for j in (0..i).rev() {
            let x2_beg = v[j].key;
            let x2_end = x2_beg + (1 << v[j].keybits) - 1;
            match cmp_chain_key_impl(x1_beg, x1_end, x2_beg, x2_end) {
                std::cmp::Ordering::Less => {
                    log::error!("bad key at {j}|{v:#?}");
                    return Err(nix::errno::Errno::EINVAL);
                }
                std::cmp::Ordering::Greater => {
                    beg = j + 1;
                    break;
                }
                std::cmp::Ordering::Equal => beg = j,
            }
        }
        let mut end = i;
        for j in (i + 1)..v.len() {
            let x2_beg = v[j].key;
            let x2_end = x2_beg + (1 << v[j].keybits) - 1;
            match cmp_chain_key_impl(x1_beg, x1_end, x2_beg, x2_end) {
                std::cmp::Ordering::Less => {
                    end = j - 1;
                    break;
                }
                std::cmp::Ordering::Greater => {
                    log::error!("bad key at {j}|{v:#?}");
                    return Err(nix::errno::Errno::EINVAL);
                }
                std::cmp::Ordering::Equal => end = j,
            }
        }
        Ok(Some((beg, end))) // end inclusive
    }

    pub(crate) fn find_child(
        &self,
        key_next: u64,
        key_beg: u64,
        key_end: u64,
    ) -> nix::Result<Option<(ChainKey, u64)>> {
        let Some((beg, end)) = self.find_child_range_impl(key_beg, key_end)? else {
            return Ok(None);
        };
        let mut key_next = key_next;
        let mut besti = usize::MAX;
        for i in beg..=end {
            if besti == usize::MAX {
                besti = i; // No previous best.  Assign best.
                continue;
            }
            let best = &self.ccids[besti];
            let child = &self.ccids[i];
            if best.key <= key_beg && child.key <= key_beg {
                // Illegal overlap.
                log::error!(
                    "illegal overlap: {:x} <= {key_beg:x} && {:x} <= {key_beg:x}",
                    best.key,
                    child.key
                );
                return Err(nix::errno::Errno::EINVAL);
            } else if child.key < best.key {
                // Child has a nearer key and best is not flush with key_beg.
                // Set best to child.  Truncate key_next to the old best key.
                besti = i;
                if key_next > best.key || key_next == 0 {
                    key_next = best.key;
                }
            } else if child.key == best.key {
                // If our current best is flush with the child then this
                // is an illegal overlap.
                //
                // key_next will automatically be limited to the smaller of
                // the two end-points.
                log::error!("illegal overlap: {:x} == {:x}", child.key, best.key);
                return Err(nix::errno::Errno::EINVAL);
            } else {
                // Keep the current best but truncate key_next to the child's
                // base.
                //
                // key_next will also automatically be limited to the smaller
                // of the two end-points (probably not necessary for this case
                // but we do it anyway).
                if key_next > child.key || key_next == 0 {
                    key_next = child.key;
                }
            }
            // Always truncate key_next based on child's end-of-range.
            let child_end = child.key + (1 << child.keybits);
            if child_end != 0 && (key_next > child_end || key_next == 0) {
                key_next = child_end;
            }
        }
        assert_ne!(besti, usize::MAX);
        Ok(Some((self.ccids[besti], key_next)))
    }

    pub(crate) fn count_blockref(&mut self) -> nix::Result<()> {
        if self.has_flags(CHAIN_COUNTED_BLOCKREF) {
            return Ok(());
        }
        let base = self.as_blockref()?;
        if base.is_empty() {
            self.live_zero = 0;
        } else {
            let mut count = base.len() - 1;
            while count != 0 {
                if base[count].typ != crate::fs::HAMMER2_BREF_TYPE_EMPTY {
                    break;
                }
                count -= 1;
            }
            self.live_zero = count + 1;
            while count != 0 {
                let base = self.as_blockref()?;
                if base[count].typ != crate::fs::HAMMER2_BREF_TYPE_EMPTY {
                    self.live_count += 1;
                }
                count -= 1;
            }
        }
        self.set_flags(CHAIN_COUNTED_BLOCKREF);
        Ok(())
    }

    pub(crate) fn find_blockref(
        &mut self,
        key_next: u64,
        key_beg: u64,
    ) -> nix::Result<(usize, u64)> {
        // Require the live chain's already have their core's counted
        // so we can optimize operations.
        assert!(self.has_flags(CHAIN_COUNTED_BLOCKREF));
        // Degenerate case
        let base = self.as_blockref()?;
        if base.is_empty() {
            return Ok((0, key_next));
        }
        // Sequential optimization using parent->cache_index.  This is
        // the most likely scenario.
        //
        // We can avoid trailing empty entries on live chains, otherwise
        // we might have to check the whole block array.
        let mut i = self.cache_index;
        let limit = self.live_zero;
        if i >= limit {
            if limit >= 1 {
                i = limit - 1;
            } else {
                i = 0;
            }
        }
        assert!(i < base.len());
        // Search backwards
        while i > 0 && (base[i].typ == crate::fs::HAMMER2_BREF_TYPE_EMPTY || base[i].key > key_beg)
        {
            i -= 1;
        }
        self.cache_index = i;
        // Search forwards, stop when we find a scan element which
        // encloses the key or until we know that there are no further
        // elements.
        let base = self.as_blockref()?;
        while i < base.len() {
            if base[i].typ != crate::fs::HAMMER2_BREF_TYPE_EMPTY {
                let scan_end = base[i].key + (1 << base[i].keybits) - 1;
                if base[i].key > key_beg || scan_end >= key_beg {
                    break;
                }
            }
            if i >= limit {
                return Ok((base.len(), key_next));
            }
            i += 1;
        }
        if i != base.len() {
            self.cache_index = i;
            let base = self.as_blockref()?;
            if i >= limit {
                i = base.len();
            } else {
                let scan_end = base[i].key + (1 << base[i].keybits);
                if scan_end != 0 && (key_next > scan_end || key_next == 0) {
                    return Ok((i, scan_end));
                }
            }
        }
        Ok((i, key_next))
    }

    // Returns true on success, false on failure.
    pub(crate) fn test_check(&mut self, bdata: &[u8]) -> nix::Result<bool> {
        if self.has_flags(CHAIN_TESTED_GOOD) {
            return Ok(true);
        }
        let r = crate::ondisk::verify_media(&self.bref, bdata)?;
        if r {
            self.set_flags(CHAIN_TESTED_GOOD);
        } else {
            log::error!("failed: chain {} flags {:08x}", self.bref, self.flags);
        }
        Ok(r)
    }

    // Returns true if the chain (INODE or DIRENT) matches the filename.
    #[must_use]
    pub fn match_name(&self, name: &str) -> bool {
        self.match_name_from_bytes(name.as_bytes())
    }

    #[must_use]
    pub fn match_name_from_bytes(&self, name: &[u8]) -> bool {
        let n = name.len();
        match self.bref.typ {
            crate::fs::HAMMER2_BREF_TYPE_INODE => {
                let ipdata = self.as_inode_data();
                usize::from(ipdata.meta.name_len) == n && ipdata.filename[..n] == *name
            }
            crate::fs::HAMMER2_BREF_TYPE_DIRENT => {
                if usize::from(self.bref.embed_as::<crate::fs::Hammer2DirentHead>().namlen) == n {
                    if n > self.bref.check.len() && self.data[..n] == *name {
                        return true;
                    }
                    if n <= self.bref.check.len() && self.bref.check[..n] == *name {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(crate) fn as_inode_data(&self) -> &crate::fs::Hammer2InodeData {
        crate::ondisk::media_as_inode_data(&self.data)
    }

    #[allow(dead_code)]
    pub(crate) fn as_volume_data(&self) -> &crate::fs::Hammer2VolumeData {
        crate::ondisk::media_as_volume_data(&self.data)
    }

    pub(crate) fn as_blockref(&self) -> nix::Result<Vec<&crate::fs::Hammer2Blockref>> {
        crate::ondisk::media_as_blockref(&self.bref, &self.data)
    }

    pub(crate) fn as_blockref_safe(&self) -> Vec<&crate::fs::Hammer2Blockref> {
        crate::ondisk::media_as_blockref_safe(&self.bref, &self.data)
    }

    pub(crate) fn read_cache_data(&mut self) -> nix::Result<Vec<u8>> {
        if !self.has_udata() {
            self.udata = self.read_data()?;
        }
        Ok(self.udata.clone())
    }

    pub(crate) fn read_data(&mut self) -> nix::Result<Vec<u8>> {
        match self.bref.typ {
            crate::fs::HAMMER2_BREF_TYPE_INODE => {
                // Ignore garbage beyond inode size.
                let ipdata = self.as_inode_data();
                Ok(ipdata.u[..ipdata.meta.size.try_into().unwrap()].to_vec())
            }
            crate::fs::HAMMER2_BREF_TYPE_DATA => self.decompress_data(),
            _ => {
                log::error!("bad blockref type {}", self.bref.typ);
                Err(nix::errno::Errno::EINVAL)
            }
        }
    }

    fn decompress_data(&self) -> nix::Result<Vec<u8>> {
        let max_size = crate::fs::HAMMER2_PBUFSIZE.try_into().unwrap();
        match crate::fs::dec_comp(self.bref.methods) {
            crate::fs::HAMMER2_COMP_NONE => {
                let n = usize::try_from(self.bytes).unwrap();
                assert!(n <= self.data.len());
                Ok(self.data[..n].to_vec())
            }
            crate::fs::HAMMER2_COMP_LZ4 => match crate::lz4::decompress(&self.data, max_size) {
                Ok(v) => Ok(v),
                Err(e) => {
                    log::error!("{e}: failed to decompress");
                    Err(nix::errno::Errno::EIO)
                }
            },
            crate::fs::HAMMER2_COMP_ZLIB => match crate::zlib::decompress(&self.data, max_size) {
                Ok(v) => Ok(v),
                Err(e) => {
                    log::error!("{e}: failed to decompress");
                    Err(nix::errno::Errno::EIO)
                }
            },
            _ => {
                log::error!("bad comp type {:02x}", self.bref.methods);
                Err(nix::errno::Errno::EINVAL)
            }
        }
    }

    pub(crate) fn get_name(&self) -> Option<String> {
        match self.bref.typ {
            crate::fs::HAMMER2_BREF_TYPE_INODE => {
                if self.has_data() {
                    self.as_inode_data().get_filename_string().ok()
                } else {
                    None
                }
            }
            crate::fs::HAMMER2_BREF_TYPE_DIRENT => {
                let s = if usize::from(self.bref.embed_as::<crate::fs::Hammer2DirentHead>().namlen)
                    <= self.bref.check.len()
                {
                    &self.bref.check.to_vec()
                } else {
                    &self.data
                };
                crate::util::bin_to_string(s).ok()
            }
            _ => None,
        }
    }
}

// Use (0x00, 0x10, 0x20) for keys rather than (0x0, 0x1, 0x2),
// otherwise delta affects the order.
#[cfg(test)]
mod tests {
    const K0: u64 = 0x00;
    const K1: u64 = 0x10;
    const K2: u64 = 0x20;

    fn alloc_empty_chain() -> super::Chain {
        match super::Chain::new(
            &crate::fs::Hammer2Blockref::new(crate::fs::HAMMER2_BREF_TYPE_INODE),
            super::CID_CHAIN_OFFSET,
        ) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        }
    }

    fn alloc_chain(cid: super::Cid, key: u64, keybits: u8) -> super::Chain {
        let mut chain = alloc_empty_chain();
        chain.cid = cid;
        chain.bref.key = key;
        chain.bref.keybits = keybits;
        chain
    }

    fn alloc_chain_key(cid: super::Cid, key: u64, keybits: u8) -> super::ChainKey {
        super::ChainKey::new(cid, key, keybits)
    }

    fn get_delta(n: u8) -> u64 {
        (1 << n) - 1 // largest delta for cmp_chain_key to return Equal
    }

    #[test]
    fn test_chain_flags() {
        let mut c = alloc_empty_chain();
        assert!(!c.has_flags(super::CHAIN_TESTED_GOOD));
        c.set_flags(super::CHAIN_TESTED_GOOD);
        assert!(c.has_flags(super::CHAIN_TESTED_GOOD));
        c.clear_flags(super::CHAIN_TESTED_GOOD);
        assert!(!c.has_flags(super::CHAIN_TESTED_GOOD));

        assert!(!c.has_flags(super::CHAIN_COUNTED_BLOCKREF));
        c.set_flags(super::CHAIN_COUNTED_BLOCKREF);
        assert!(c.has_flags(super::CHAIN_COUNTED_BLOCKREF));
        c.clear_flags(super::CHAIN_COUNTED_BLOCKREF);
        assert!(!c.has_flags(super::CHAIN_COUNTED_BLOCKREF));
    }

    fn test_cmp_chain_key(n: u8) {
        let d = get_delta(n);
        for k in [K0, K1, K2] {
            let x1 = alloc_chain_key(0, k << n, n);
            let x2 = alloc_chain_key(0, (k << n) + d, n);
            assert!(
                super::cmp_chain_key(&x1, &x2) == std::cmp::Ordering::Equal,
                "{k}: {x1:#?} vs {x2:#?}"
            );
        }
        let d = get_delta(n) + 1;
        for k in [K0, K1, K2] {
            let x1 = alloc_chain_key(0, k << n, n);
            let x2 = alloc_chain_key(0, (k << n) + d, n);
            assert!(
                super::cmp_chain_key(&x1, &x2) == std::cmp::Ordering::Less,
                "{k}: {x1:#?} vs {x2:#?}"
            );
        }
        let d = get_delta(n) + 1;
        for k in [K0, K1, K2] {
            let x1 = alloc_chain_key(0, (k << n) + d, n);
            let x2 = alloc_chain_key(0, k << n, n);
            assert!(
                super::cmp_chain_key(&x1, &x2) == std::cmp::Ordering::Greater,
                "{k}: {x1:#?} vs {x2:#?}"
            );
        }
    }

    #[test]
    fn test_cmp_chain_key_keybits_0() {
        test_cmp_chain_key(0);
    }

    #[test]
    fn test_cmp_chain_key_keybits_1() {
        test_cmp_chain_key(1);
    }

    #[test]
    fn test_cmp_chain_key_keybits_8() {
        test_cmp_chain_key(8);
    }

    #[test]
    fn test_cmp_chain_key_keybits_16() {
        test_cmp_chain_key(16);
    }

    fn test_chain_get_child(n: u8) {
        let c = alloc_empty_chain();
        assert_eq!(c.get_first_child(), None);
        assert!(c.get_child().is_empty());

        let d = get_delta(n);
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, (K0 << n) + d, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, (K1 << n) + d, n));
        c.add_child(&alloc_chain(4, K2 << n, n));
        c.add_child(&alloc_chain(5, (K2 << n) + d, n));
        assert_eq!(c.get_first_child(), Some(0));
        let v = c.get_child();
        assert_eq!(v[0], 0, "{v:#?}");
        assert_eq!(v[1], 1, "{v:#?}");
        assert_eq!(v[2], 2, "{v:#?}");
        assert_eq!(v[3], 3, "{v:#?}");
        assert_eq!(v[4], 4, "{v:#?}");
        assert_eq!(v[5], 5, "{v:#?}");

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, (K2 << n) + d, n));
        c.add_child(&alloc_chain(1, K2 << n, n));
        c.add_child(&alloc_chain(2, (K1 << n) + d, n));
        c.add_child(&alloc_chain(3, K1 << n, n));
        c.add_child(&alloc_chain(4, (K0 << n) + d, n));
        c.add_child(&alloc_chain(5, K0 << n, n));
        assert_eq!(c.get_first_child(), Some(4));
        let v = c.get_child();
        assert_eq!(v[0], 4, "{v:#?}");
        assert_eq!(v[1], 5, "{v:#?}");
        assert_eq!(v[2], 2, "{v:#?}");
        assert_eq!(v[3], 3, "{v:#?}");
        assert_eq!(v[4], 0, "{v:#?}");
        assert_eq!(v[5], 1, "{v:#?}");

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K1 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, K1 << n, n));
        c.add_child(&alloc_chain(4, K1 << n, n));
        c.add_child(&alloc_chain(5, K1 << n, n));
        assert_eq!(c.get_first_child(), Some(0));
        let v = c.get_child();
        assert_eq!(v[0], 0, "{v:#?}");
        assert_eq!(v[1], 1, "{v:#?}");
        assert_eq!(v[2], 2, "{v:#?}");
        assert_eq!(v[3], 3, "{v:#?}");
        assert_eq!(v[4], 4, "{v:#?}");
        assert_eq!(v[5], 5, "{v:#?}");
    }

    #[test]
    fn test_chain_get_child_keybits_0() {
        test_chain_get_child(0);
    }

    #[test]
    fn test_chain_get_child_keybits_1() {
        test_chain_get_child(1);
    }

    #[test]
    fn test_chain_get_child_keybits_8() {
        test_chain_get_child(8);
    }

    #[test]
    fn test_chain_get_child_keybits_16() {
        test_chain_get_child(16);
    }

    fn test_chain_add_child(n: u8) {
        let d = get_delta(n);
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, (K0 << n) + d, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, (K1 << n) + d, n));
        c.add_child(&alloc_chain(4, K2 << n, n));
        c.add_child(&alloc_chain(5, (K2 << n) + d, n));
        let v = &c.ccids;
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, (K0 << n) + d), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K1 << n), "{v:#?}");
        assert_eq!((v[3].cid, v[3].key), (3, (K1 << n) + d), "{v:#?}");
        assert_eq!((v[4].cid, v[4].key), (4, K2 << n), "{v:#?}");
        assert_eq!((v[5].cid, v[5].key), (5, (K2 << n) + d), "{v:#?}");

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, (K2 << n) + d, n));
        c.add_child(&alloc_chain(1, K2 << n, n));
        c.add_child(&alloc_chain(2, (K1 << n) + d, n));
        c.add_child(&alloc_chain(3, K1 << n, n));
        c.add_child(&alloc_chain(4, (K0 << n) + d, n));
        c.add_child(&alloc_chain(5, K0 << n, n));
        let v = &c.ccids;
        assert_eq!((v[0].cid, v[0].key), (4, (K0 << n) + d), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (5, K0 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, (K1 << n) + d), "{v:#?}");
        assert_eq!((v[3].cid, v[3].key), (3, K1 << n), "{v:#?}");
        assert_eq!((v[4].cid, v[4].key), (0, (K2 << n) + d), "{v:#?}");
        assert_eq!((v[5].cid, v[5].key), (1, K2 << n), "{v:#?}");

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K1 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, K1 << n, n));
        c.add_child(&alloc_chain(4, K1 << n, n));
        c.add_child(&alloc_chain(5, K1 << n, n));
        let v = &c.ccids;
        assert_eq!((v[0].cid, v[0].key), (0, K1 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K1 << n), "{v:#?}");
        assert_eq!((v[3].cid, v[3].key), (3, K1 << n), "{v:#?}");
        assert_eq!((v[4].cid, v[4].key), (4, K1 << n), "{v:#?}");
        assert_eq!((v[5].cid, v[5].key), (5, K1 << n), "{v:#?}");
    }

    #[test]
    fn test_chain_add_child_keybits_0() {
        test_chain_add_child(0);
    }

    #[test]
    fn test_chain_add_child_keybits_1() {
        test_chain_add_child(1);
    }

    #[test]
    fn test_chain_add_child_keybits_8() {
        test_chain_add_child(8);
    }

    #[test]
    fn test_chain_add_child_keybits_16() {
        test_chain_add_child(16);
    }

    #[allow(clippy::too_many_lines)]
    fn test_chain_remove_child(n: u8) {
        // 0 1 2
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K2 << n, n));
        let v = &c.ccids;
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(0) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].cid, v[0].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(1) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].cid, v[0].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(2) {
            panic!("{e}: {:#?}", c.ccids);
        }
        assert!(!c.has_child());
        assert_eq!(c.remove_child(0), Err(nix::errno::Errno::ENOENT));

        // 0 2 1
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K2 << n, n));
        let v = &c.ccids;
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(0) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].cid, v[0].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(2) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].cid, v[0].key), (1, K1 << n), "{v:#?}");
        if let Err(e) = c.remove_child(1) {
            panic!("{e}: {:#?}", c.ccids);
        }
        assert!(!c.has_child());
        assert_eq!(c.remove_child(0), Err(nix::errno::Errno::ENOENT));

        // 1 0 2
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K2 << n, n));
        let v = &c.ccids;
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(1) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(0) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].cid, v[0].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(2) {
            panic!("{e}: {:#?}", c.ccids);
        }
        assert!(!c.has_child());
        assert_eq!(c.remove_child(0), Err(nix::errno::Errno::ENOENT));

        // 1 2 0
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K2 << n, n));
        let v = &c.ccids;
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(1) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(2) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        if let Err(e) = c.remove_child(0) {
            panic!("{e}: {:#?}", c.ccids);
        }
        assert!(!c.has_child());
        assert_eq!(c.remove_child(0), Err(nix::errno::Errno::ENOENT));

        // 2 0 1
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K2 << n, n));
        let v = &c.ccids;
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(2) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        if let Err(e) = c.remove_child(0) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].cid, v[0].key), (1, K1 << n), "{v:#?}");
        if let Err(e) = c.remove_child(1) {
            panic!("{e}: {:#?}", c.ccids);
        }
        assert!(!c.has_child());
        assert_eq!(c.remove_child(0), Err(nix::errno::Errno::ENOENT));

        // 2 1 0
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K2 << n, n));
        let v = &c.ccids;
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        assert_eq!((v[2].cid, v[2].key), (2, K2 << n), "{v:#?}");
        if let Err(e) = c.remove_child(2) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        assert_eq!((v[1].cid, v[1].key), (1, K1 << n), "{v:#?}");
        if let Err(e) = c.remove_child(1) {
            panic!("{e}: {:#?}", c.ccids);
        }
        let v = &c.ccids;
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].cid, v[0].key), (0, K0 << n), "{v:#?}");
        if let Err(e) = c.remove_child(0) {
            panic!("{e}: {:#?}", c.ccids);
        }
        assert!(!c.has_child());
        assert_eq!(c.remove_child(0), Err(nix::errno::Errno::ENOENT));
    }

    #[test]
    fn test_chain_remove_child_keybits_0() {
        test_chain_remove_child(0);
    }

    #[test]
    fn test_chain_remove_child_keybits_1() {
        test_chain_remove_child(1);
    }

    #[test]
    fn test_chain_remove_child_keybits_8() {
        test_chain_remove_child(8);
    }

    #[test]
    fn test_chain_remove_child_keybits_16() {
        test_chain_remove_child(16);
    }

    #[allow(clippy::many_single_char_names)]
    fn test_chain_find_child_index(n: u8) {
        let d = get_delta(n);
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, (K0 << n) + d, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, (K1 << n) + d, n));
        c.add_child(&alloc_chain(4, K2 << n, n));
        c.add_child(&alloc_chain(5, (K2 << n) + d, n));
        let x = alloc_chain_key(0, K1 << n, n);
        let Some(i) = c.find_child_index(&x) else {
            panic!("{x:#?}")
        };
        assert!((2..=3).contains(&i), "{:#?}", c.ccids);
        let v = &c.ccids;
        assert_eq!((v[i].key, v[i].keybits), (x.key, x.keybits), "{v:#?}");
        assert!(
            super::cmp_chain_key(&v[i], &x) == std::cmp::Ordering::Equal,
            "{:#?} vs {x:#?}",
            v[i]
        );

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K1 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, K1 << n, n));
        c.add_child(&alloc_chain(4, K1 << n, n));
        c.add_child(&alloc_chain(5, K1 << n, n));
        let x = alloc_chain_key(0, K1 << n, n);
        let Some(i) = c.find_child_index(&x) else {
            panic!("{x:#?}")
        };
        assert!((0..=5).contains(&i), "{:#?}", c.ccids);
        let v = &c.ccids;
        assert_eq!((v[i].key, v[i].keybits), (x.key, x.keybits), "{v:#?}");
        assert!(
            super::cmp_chain_key(&v[i], &x) == std::cmp::Ordering::Equal,
            "{:#?} vs {x:#?}",
            v[i]
        );
    }

    #[test]
    fn test_chain_find_child_index_keybits_0() {
        test_chain_find_child_index(0);
    }

    #[test]
    fn test_chain_find_child_index_keybits_1() {
        test_chain_find_child_index(1);
    }

    #[test]
    fn test_chain_find_child_index_keybits_8() {
        test_chain_find_child_index(8);
    }

    #[test]
    fn test_chain_find_child_index_keybits_16() {
        test_chain_find_child_index(16);
    }

    #[allow(clippy::many_single_char_names)]
    fn test_chain_find_child_range(n: u8) {
        let d = get_delta(n);
        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, (K0 << n) + d, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, (K1 << n) + d, n));
        c.add_child(&alloc_chain(4, K2 << n, n));
        c.add_child(&alloc_chain(5, (K2 << n) + d, n));
        let x = alloc_chain_key(0, K0 << n, n);
        let (beg, end) = match c.find_child_range(&x) {
            Ok(v) => match v {
                Some(v) => v,
                None => panic!("{x:#?}"),
            },
            Err(e) => panic!("{e}"),
        };
        assert_eq!((beg, end), (0, 1), "{:#?}", c.ccids);
        for c in c.ccids.iter().take(end + 1).skip(beg) {
            assert!(
                super::cmp_chain_key(c, &x) == std::cmp::Ordering::Equal,
                "{c:#?} vs {x:#?}"
            );
        }

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, (K0 << n) + d, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, (K1 << n) + d, n));
        c.add_child(&alloc_chain(4, K2 << n, n));
        c.add_child(&alloc_chain(5, (K2 << n) + d, n));
        let x = alloc_chain_key(0, K1 << n, n);
        let (beg, end) = match c.find_child_range(&x) {
            Ok(v) => match v {
                Some(v) => v,
                None => panic!("{x:#?}"),
            },
            Err(e) => panic!("{e}"),
        };
        assert_eq!((beg, end), (2, 3), "{:#?}", c.ccids);
        for c in c.ccids.iter().take(end + 1).skip(beg) {
            assert!(
                super::cmp_chain_key(c, &x) == std::cmp::Ordering::Equal,
                "{c:#?} vs {x:#?}"
            );
        }

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K0 << n, n));
        c.add_child(&alloc_chain(1, (K0 << n) + d, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, (K1 << n) + d, n));
        c.add_child(&alloc_chain(4, K2 << n, n));
        c.add_child(&alloc_chain(5, (K2 << n) + d, n));
        let x = alloc_chain_key(0, K2 << n, n);
        let (beg, end) = match c.find_child_range(&x) {
            Ok(v) => match v {
                Some(v) => v,
                None => panic!("{x:#?}"),
            },
            Err(e) => panic!("{e}"),
        };
        assert_eq!((beg, end), (4, 5), "{:#?}", c.ccids);
        for c in c.ccids.iter().take(end + 1).skip(beg) {
            assert!(
                super::cmp_chain_key(c, &x) == std::cmp::Ordering::Equal,
                "{c:#?} vs {x:#?}"
            );
        }

        let mut c = alloc_empty_chain();
        c.add_child(&alloc_chain(0, K1 << n, n));
        c.add_child(&alloc_chain(1, K1 << n, n));
        c.add_child(&alloc_chain(2, K1 << n, n));
        c.add_child(&alloc_chain(3, K1 << n, n));
        c.add_child(&alloc_chain(4, K1 << n, n));
        c.add_child(&alloc_chain(5, K1 << n, n));
        let x = alloc_chain_key(0, K1 << n, n);
        let (beg, end) = match c.find_child_range(&x) {
            Ok(v) => match v {
                Some(v) => v,
                None => panic!("{x:#?}"),
            },
            Err(e) => panic!("{e}"),
        };
        assert_eq!((beg, end), (0, 5), "{:#?}", c.ccids);
        for c in c.ccids.iter().take(end + 1).skip(beg) {
            assert!(
                super::cmp_chain_key(c, &x) == std::cmp::Ordering::Equal,
                "{c:#?} vs {x:#?}"
            );
        }
    }

    #[test]
    fn test_chain_find_child_range_keybits_0() {
        test_chain_find_child_range(0);
    }

    #[test]
    fn test_chain_find_child_range_keybits_1() {
        test_chain_find_child_range(1);
    }

    #[test]
    fn test_chain_find_child_range_keybits_8() {
        test_chain_find_child_range(8);
    }

    #[test]
    fn test_chain_find_child_range_keybits_16() {
        test_chain_find_child_range(16);
    }
}
