use std::io::Read;
use std::io::Write;

#[derive(Debug)]
pub struct Volume {
    id: u8,
    path: String,
    fp: std::fs::File,
    offset: u64,
    size: u64,
}

impl Drop for Volume {
    fn drop(&mut self) {
        self.fsync().unwrap();
    }
}

impl Volume {
    /// # Errors
    pub fn new(id: u8, path: &str, readonly: bool, offset: u64, size: u64) -> crate::Result<Self> {
        Ok(Self {
            id,
            path: path.to_string(),
            fp: crate::util::open(path, readonly)?,
            offset,
            size,
        })
    }

    #[must_use]
    pub fn get_id(&self) -> usize {
        self.id.into()
    }

    #[must_use]
    pub fn get_path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn get_offset(&self) -> u64 {
        self.offset
    }

    #[must_use]
    pub fn get_size(&self) -> u64 {
        self.size
    }

    /// # Errors
    pub fn fsync(&mut self) -> std::io::Result<()> {
        self.fp.flush()
    }

    /// # Errors
    /// # Panics
    pub fn pread(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        assert_eq!(offset & (crate::fs::HAMMER2_ALLOC_MIN as u64 - 1), 0);
        crate::util::seek_set(&mut self.fp, offset)?;
        self.fp.read_exact(buf)
    }

    /// # Errors
    /// # Panics
    pub fn pwrite(&mut self, buf: &[u8], offset: u64) -> std::io::Result<()> {
        assert_eq!(offset & (crate::fs::HAMMER2_ALLOC_MIN as u64 - 1), 0);
        crate::util::seek_set(&mut self.fp, offset)?;
        self.fp.write_all(buf)
    }

    /// # Errors
    /// # Panics
    pub fn preadx(&mut self, size: u64, offset: u64) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0; size.try_into().unwrap()];
        self.pread(&mut buf, offset)?;
        Ok(buf)
    }
}

// get volume data offset relative to a volume
/// # Panics
#[must_use]
pub fn get_volume_data_offset(index: usize) -> u64 {
    u64::try_from(index).unwrap() * crate::fs::HAMMER2_ZONE_BYTES
}

// Locate a valid volume header.  If any of the four volume headers is good,
// we have a valid volume header and choose the best one based on mirror_tid.
pub(crate) fn read_volume_data(path: &str) -> crate::Result<crate::fs::Hammer2VolumeData> {
    let mut fp = crate::util::open(path, false)?;
    let size = crate::subs::get_volume_size(&mut fp)?;
    let mut zone = usize::MAX;
    let mut mirror_tid = u64::MAX;
    let mut v = vec![];

    for i in 0..crate::fs::HAMMER2_NUM_VOLHDRS {
        let offset = get_volume_data_offset(i);
        if offset >= size {
            break;
        }
        crate::util::seek_set(&mut fp, offset)?;
        let mut buf = vec![0; crate::fs::HAMMER2_VOLUME_BYTES as usize];
        fp.read_exact(&mut buf)?;
        let vd = crate::util::align_to::<crate::fs::Hammer2VolumeData>(&buf);
        // verify volume header magic
        if vd.magic != crate::fs::HAMMER2_VOLUME_ID_HBO
            && vd.magic != crate::fs::HAMMER2_VOLUME_ID_ABO
        {
            log::error!("{path} #{i}: bad magic");
            continue;
        }
        if vd.magic == crate::fs::HAMMER2_VOLUME_ID_ABO {
            // XXX: Reversed-endianness filesystem
            log::error!("{path} #{i}: reverse-endian filesystem detected");
            continue;
        }
        // verify volume header CRC's
        let a = vd.icrc_sects[crate::fs::HAMMER2_VOL_ICRC_SECT0];
        let b = vd.get_crc(
            crate::fs::HAMMER2_VOLUME_ICRC0_OFF,
            crate::fs::HAMMER2_VOLUME_ICRC0_SIZE,
        );
        if a != b {
            log::error!("{path} #{i}: volume header crc mismatch sect0 {a:08x}/{b:08x}");
            continue;
        }
        let a = vd.icrc_sects[crate::fs::HAMMER2_VOL_ICRC_SECT1];
        let b = vd.get_crc(
            crate::fs::HAMMER2_VOLUME_ICRC1_OFF,
            crate::fs::HAMMER2_VOLUME_ICRC1_SIZE,
        );
        if a != b {
            log::error!("{path} #{i}: volume header crc mismatch sect1 {a:08x}/{b:08x}");
            continue;
        }
        let a = vd.icrc_volheader;
        let b = vd.get_crc(
            crate::fs::HAMMER2_VOLUME_ICRCVH_OFF,
            crate::fs::HAMMER2_VOLUME_ICRCVH_SIZE,
        );
        if a != b {
            log::error!("{path} #{i}: volume header crc mismatch vh {a:08x}/{b:08x}");
            continue;
        }
        if zone == usize::MAX || mirror_tid < vd.mirror_tid {
            zone = i;
            mirror_tid = vd.mirror_tid;
            v.push(*vd);
        }
    }
    if zone == usize::MAX {
        Err(nix::errno::Errno::ENOENT.into())
    } else {
        Ok(v[zone])
    }
}
