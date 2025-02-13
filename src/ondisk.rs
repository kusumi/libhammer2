use crate::os::MetadataExt;
use std::os::unix::fs::FileTypeExt;

#[derive(Debug, Default)]
struct VolumeIdentifier {
    version: u32,
    nvolumes: u8,
    fsid: [u8; 16],
    fstype: [u8; 16],
}

impl VolumeIdentifier {
    fn new(version: Option<u32>) -> Self {
        Self {
            version: version.unwrap_or(crate::fs::HAMMER2_VOL_VERSION_DEFAULT),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct Ondisk {
    volumes: Vec<crate::volume::Volume>,
    total_size: u64,
    ident: VolumeIdentifier, // mostly unused by newfs_hammer2
}

impl std::ops::Index<usize> for Ondisk {
    type Output = crate::volume::Volume;
    fn index(&self, i: usize) -> &Self::Output {
        self.volumes.index(i)
    }
}

impl std::ops::IndexMut<usize> for Ondisk {
    fn index_mut(&mut self, i: usize) -> &mut crate::volume::Volume {
        self.volumes.index_mut(i)
    }
}

impl Ondisk {
    #[must_use]
    pub fn new(version: Option<u32>) -> Self {
        Self {
            ident: VolumeIdentifier::new(version),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn get_nvolumes(&self) -> usize {
        self.volumes.len()
    }

    #[must_use]
    pub fn get_total_size(&self) -> u64 {
        self.total_size
    }

    /// # Errors
    pub fn install_volume(
        &mut self,
        id: u8,
        path: &str,
        readonly: bool,
        offset: u64,
        size: u64,
    ) -> crate::Result<()> {
        let vol = crate::volume::Volume::new(id, path, readonly, offset, size)?;
        self.volumes.push(vol);
        self.volumes.sort_by_key(crate::volume::Volume::get_id);
        self.total_size += size;
        Ok(())
    }

    /// # Errors
    pub fn add_volume(&mut self, path: &str, readonly: bool) -> crate::Result<()> {
        let t = std::fs::metadata(path)?.file_type();
        if !t.is_block_device() && !t.is_char_device() && !t.is_file() {
            log::error!("Unsupported file type {t:?}");
            return Err(nix::errno::Errno::EINVAL.into());
        }
        if self.volumes.len() >= crate::fs::HAMMER2_MAX_VOLUMES.into() {
            log::error!(
                "Exceeds maximum supported number of volumes {}",
                crate::fs::HAMMER2_MAX_VOLUMES
            );
            return Err(nix::errno::Errno::EINVAL.into());
        }
        let voldata = crate::volume::read_volume_data(path)?;
        if voldata.volu_id >= crate::fs::HAMMER2_MAX_VOLUMES {
            log::error!("{path} has bad volume id {}", voldata.volu_id);
            return Err(nix::errno::Errno::EINVAL.into());
        }
        // all headers must have the same version, nvolumes and uuid
        if self.ident.nvolumes == 0 {
            self.ident.version = voldata.version;
            self.ident.nvolumes = voldata.nvolumes;
            self.ident.fsid = voldata.fsid;
            self.ident.fstype = voldata.fstype;
        } else {
            if self.ident.version != voldata.version {
                log::error!(
                    "Volume version mismatch {} vs {}",
                    self.ident.version,
                    voldata.version
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
            if self.ident.nvolumes != voldata.nvolumes {
                log::error!(
                    "Volume count mismatch {} vs {}",
                    self.ident.nvolumes,
                    voldata.nvolumes
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
            if self.ident.fsid != voldata.fsid {
                log::error!(
                    "Volume fsid UUID mismatch {:?} vs {:?}",
                    self.ident.fsid,
                    voldata.fsid
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
            if self.ident.fstype != voldata.fstype {
                log::error!(
                    "Volume fstype UUID mismatch {:?} vs {:?}",
                    self.ident.fstype,
                    voldata.fstype
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
        }
        // all per-volume tests passed
        self.install_volume(
            voldata.volu_id,
            path,
            readonly,
            voldata.volu_loff[usize::from(voldata.volu_id)],
            voldata.volu_size,
        )?;
        Ok(())
    }

    fn verify_volumes_common(&self, verify_rootvol: bool) -> crate::Result<()> {
        // check volume header
        if verify_rootvol {
            let rootvoldata = self.read_root_volume_data()?;
            if rootvoldata.volu_id != crate::fs::HAMMER2_ROOT_VOLUME {
                log::error!(
                    "Volume id {} must be {}",
                    rootvoldata.volu_id,
                    crate::fs::HAMMER2_ROOT_VOLUME
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
            if crate::subs::get_uuid_string_from_bytes(&rootvoldata.fstype)
                != crate::fs::HAMMER2_UUID_STRING
            {
                log::error!(
                    "Volume fstype UUID {:?} must be {}",
                    rootvoldata.fstype,
                    crate::fs::HAMMER2_UUID_STRING
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
        }
        let mut st = vec![];
        for (i, vol) in self.volumes.iter().enumerate() {
            assert!(vol.get_id() < crate::fs::HAMMER2_MAX_VOLUMES.into());
            // check volumes are unique
            st.push(std::fs::metadata(vol.get_path())?);
            for j in 0..i {
                if st[i].st_ino() == st[j].st_ino() && st[i].st_dev() == st[j].st_dev() {
                    log::error!("{} specified more than once", vol.get_path());
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
            // check volume size vs block device size
            let size = crate::subs::get_volume_size_from_path(vol.get_path())?;
            println!("checkvolu header {i} {:016x}/{:016x}", vol.get_size(), size);
            if vol.get_size() > size {
                log::error!(
                    "{}'s size {:#018x} exceeds device size {:#018x}",
                    vol.get_path(),
                    vol.get_size(),
                    size
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
            if vol.get_size() == 0 {
                log::error!("{} has size of 0", vol.get_path());
                return Err(nix::errno::Errno::EINVAL.into());
            }
        }
        Ok(())
    }

    fn verify_volumes_1(&self, verify_rootvol: bool) -> crate::Result<()> {
        // check initialized volume count
        if self.volumes.len() != 1 {
            log::error!("Only 1 volume supported");
            return Err(nix::errno::Errno::EINVAL.into());
        }
        // check volume header
        if verify_rootvol {
            let rootvoldata = self.read_root_volume_data()?;
            if rootvoldata.nvolumes != 0 {
                log::error!("Volume count {} must be 0", rootvoldata.nvolumes);
                return Err(nix::errno::Errno::EINVAL.into());
            }
            if rootvoldata.total_size != 0 {
                log::error!("Total size {:#018x} must be 0", rootvoldata.total_size);
                return Err(nix::errno::Errno::EINVAL.into());
            }
            for i in 0..crate::fs::HAMMER2_MAX_VOLUMES.into() {
                let off = rootvoldata.volu_loff[i];
                if off != 0 {
                    log::error!("Volume offset[{}] {:#018x} must be 0", i, off);
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
        }
        // check volume
        let vol = &self.volumes[usize::from(crate::fs::HAMMER2_ROOT_VOLUME)];
        if vol.get_id() != 0 {
            log::error!("{} has non zero id {}", vol.get_path(), vol.get_id());
            return Err(nix::errno::Errno::EINVAL.into());
        }
        if vol.get_offset() != 0 {
            log::error!(
                "{} has non zero offset {:#018x}",
                vol.get_path(),
                vol.get_offset()
            );
            return Err(nix::errno::Errno::EINVAL.into());
        }
        if vol.get_size() & crate::fs::HAMMER2_VOLUME_ALIGNMASK != 0 {
            log::error!(
                "{}'s size is not {:#018x} aligned",
                vol.get_path(),
                crate::fs::HAMMER2_VOLUME_ALIGN
            );
            return Err(nix::errno::Errno::EINVAL.into());
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn verify_volumes_2(&self, verify_rootvol: bool) -> crate::Result<()> {
        // check volume header
        if verify_rootvol {
            let rootvoldata = self.read_root_volume_data()?;
            let nvolumes = self.get_nvolumes();
            if usize::from(rootvoldata.nvolumes) != nvolumes {
                log::error!(
                    "Volume header requires {} devices, {} specified",
                    rootvoldata.nvolumes,
                    nvolumes
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
            if rootvoldata.total_size != self.total_size {
                log::error!(
                    "Total size {:#018x} does not equal sum of volumes {:#018x}",
                    rootvoldata.total_size,
                    self.total_size
                );
                return Err(nix::errno::Errno::EINVAL.into());
            }
            for i in 0..nvolumes {
                let off = rootvoldata.volu_loff[i];
                if off == u64::MAX {
                    log::error!(
                        "Volume offset[{}] {:#018x} must not be {:#018x}",
                        i,
                        off,
                        u64::MAX
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
            for i in nvolumes..crate::fs::HAMMER2_MAX_VOLUMES.into() {
                let off = rootvoldata.volu_loff[i];
                if off != u64::MAX {
                    log::error!(
                        "Volume offset[{}] {:#018x} must be {:#018x}",
                        i,
                        off,
                        u64::MAX
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
        }
        // check volumes
        for (i, vol) in self.volumes.iter().enumerate() {
            assert!(vol.get_id() < crate::fs::HAMMER2_MAX_VOLUMES.into());
            // check offset
            if vol.get_offset() & crate::fs::HAMMER2_FREEMAP_LEVEL1_MASK != 0 {
                log::error!(
                    "{}'s offset {:#018x} not {:#018x} aligned",
                    vol.get_path(),
                    vol.get_offset(),
                    crate::fs::HAMMER2_FREEMAP_LEVEL1_SIZE
                );
            }
            // check vs previous volume
            if i > 0 {
                let prev = &self.volumes[i - 1];
                if vol.get_id() != prev.get_id() + 1 {
                    log::error!("{} has inconsistent id {}", vol.get_path(), vol.get_id());
                    return Err(nix::errno::Errno::EINVAL.into());
                }
                if vol.get_offset() != prev.get_offset() + prev.get_size() {
                    log::error!(
                        "{} has inconsistent offset {}",
                        vol.get_path(),
                        vol.get_offset()
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            } else {
                // first
                if vol.get_offset() != 0 {
                    log::error!(
                        "{} has non zero offset {:#018x}",
                        vol.get_path(),
                        vol.get_offset()
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
            // check size for non-last and last volumes
            if i != self.volumes.len() - 1 {
                if vol.get_size() < crate::fs::HAMMER2_FREEMAP_LEVEL1_SIZE {
                    log::error!(
                        "{}'s size must be >= {:#018x}",
                        vol.get_path(),
                        crate::fs::HAMMER2_FREEMAP_LEVEL1_SIZE
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
                if vol.get_size() & crate::fs::HAMMER2_FREEMAP_LEVEL1_MASK != 0 {
                    log::error!(
                        "{}'s size is not {:#018x} aligned",
                        vol.get_path(),
                        crate::fs::HAMMER2_FREEMAP_LEVEL1_SIZE
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            } else {
                // last
                if vol.get_size() & crate::fs::HAMMER2_VOLUME_ALIGNMASK != 0 {
                    log::error!(
                        "{}'s size is not {:#018x} aligned",
                        vol.get_path(),
                        crate::fs::HAMMER2_VOLUME_ALIGN
                    );
                    return Err(nix::errno::Errno::EINVAL.into());
                }
            }
        }
        Ok(())
    }

    /// # Errors
    pub fn verify_volumes(&self, verify_rootvol: bool) -> crate::Result<()> {
        self.verify_volumes_common(verify_rootvol)?;
        if self.ident.version >= crate::fs::HAMMER2_VOL_VERSION_MULTI_VOLUMES {
            self.verify_volumes_2(verify_rootvol)
        } else {
            self.verify_volumes_1(verify_rootvol)
        }
    }

    pub fn print_volumes(&self) {
        for s in &self.format_volumes() {
            println!("{s}");
        }
    }

    pub fn log_volumes(&self) {
        for s in &self.format_volumes() {
            log::info!("{s}");
        }
    }

    fn format_volumes(&self) -> Vec<String> {
        let mut w = 0;
        for vol in &self.volumes {
            let n = vol.get_path().len();
            if n > w {
                w = n;
            }
        }
        let mut v = vec![];
        v.push(format!(
            "total    {} {:#018x} {:#018x}",
            " ".repeat(w),
            0,
            self.get_total_size()
        ));
        for vol in &self.volumes {
            let s = if vol.get_id() == crate::fs::HAMMER2_ROOT_VOLUME.into() {
                " (root volume)"
            } else {
                ""
            };
            v.push(format!(
                "volume{:<2} {:<w$} {:#018x} {:#018x}{}",
                vol.get_id(),
                vol.get_path(),
                vol.get_offset(),
                vol.get_size(),
                s
            ));
        }
        v
    }

    #[must_use]
    pub fn get_volume(&self, offset: u64) -> Option<&crate::volume::Volume> {
        let mut offset = offset;
        offset &= !crate::fs::HAMMER2_OFF_MASK_RADIX;
        self.volumes
            .iter()
            .find(|&vol| offset >= vol.get_offset() && offset < vol.get_offset() + vol.get_size())
    }

    #[must_use]
    pub fn get_volume_mut(&mut self, offset: u64) -> Option<&mut crate::volume::Volume> {
        let mut offset = offset;
        offset &= !crate::fs::HAMMER2_OFF_MASK_RADIX;
        self.volumes
            .iter_mut()
            .find(|vol| offset >= vol.get_offset() && offset < vol.get_offset() + vol.get_size())
    }

    #[must_use]
    pub fn get_root_volume(&self) -> Option<&crate::volume::Volume> {
        self.get_volume(0)
    }

    #[must_use]
    pub fn get_root_volume_mut(&mut self) -> Option<&mut crate::volume::Volume> {
        self.get_volume_mut(0)
    }

    fn read_root_volume_data(&self) -> crate::Result<crate::fs::Hammer2VolumeData> {
        crate::volume::read_volume_data(
            self.get_root_volume()
                .ok_or(nix::errno::Errno::ENOENT)?
                .get_path(),
        )
    }

    /// # Errors
    /// # Panics
    pub fn get_best_volume_data(
        &mut self,
    ) -> crate::Result<Vec<(usize, crate::fs::Hammer2VolumeData)>> {
        let mut bests = vec![];
        for i in 0..self.get_nvolumes() {
            let vol = &mut self.volumes[i];
            let mut index = usize::MAX;
            let mut best = crate::fs::Hammer2VolumeData::new();
            for j in 0..crate::fs::HAMMER2_NUM_VOLHDRS {
                let offset = crate::volume::get_volume_data_offset(j);
                if offset < vol.get_size() {
                    let buf = vol.preadx(crate::fs::HAMMER2_VOLUME_BYTES, offset)?;
                    let voldata = crate::util::align_to::<crate::fs::Hammer2VolumeData>(&buf);
                    assert!(
                        voldata.magic == crate::fs::HAMMER2_VOLUME_ID_HBO
                            || voldata.magic == crate::fs::HAMMER2_VOLUME_ID_ABO
                    );
                    if j == 0 || best.mirror_tid < voldata.mirror_tid {
                        index = j;
                        best = *voldata;
                    }
                }
            }
            bests.push((index, best));
        }
        for best in &bests {
            assert_ne!(best.0, usize::MAX);
            assert_ne!(best.1.mirror_tid, 0);
        }
        Ok(bests)
    }

    /// # Errors
    /// # Panics
    pub fn read_media(&mut self, bref: &crate::fs::Hammer2Blockref) -> crate::Result<Vec<u8>> {
        let radix = bref.data_off & crate::fs::HAMMER2_OFF_MASK_RADIX;
        let bytes = if radix == 0 { 0 } else { 1 << radix };
        if bytes == 0 {
            return Ok(vec![]);
        }
        let io_off = bref.data_off & !crate::fs::HAMMER2_OFF_MASK_RADIX;
        let io_base = io_off & !crate::fs::HAMMER2_LBUFMASK;
        let boff = io_off - io_base;
        let mut io_bytes = crate::fs::HAMMER2_LBUFSIZE;
        while io_bytes + boff < bytes {
            io_bytes <<= 1;
        }
        if io_bytes > crate::fs::HAMMER2_PBUFSIZE {
            return Err(nix::errno::Errno::EINVAL.into());
        }
        let vol = self
            .get_volume_mut(io_off)
            .ok_or::<crate::Error>(nix::errno::Errno::ENOENT.into())?;
        let beg = usize::try_from(boff).unwrap();
        let end = usize::try_from(boff + bytes).unwrap();
        Ok(vol.preadx(io_bytes, io_base - vol.get_offset())?[beg..end].to_vec())
    }
}

/// # Errors
pub fn init(blkdevs: &str, readonly: bool) -> crate::Result<Ondisk> {
    let mut fso = Ondisk::new(None);
    for s in &blkdevs.split(':').collect::<Vec<&str>>() {
        fso.add_volume(s, readonly)?;
    }
    fso.verify_volumes(true)?;
    Ok(fso)
}

#[cfg(test)]
mod tests {
    const HAMMER2_BLKDEVS: &str = "HAMMER2_BLKDEVS";

    fn init_std_logger() -> Result<(), log::SetLoggerError> {
        let env = env_logger::Env::default().filter_or("RUST_LOG", "trace");
        env_logger::try_init_from_env(env)
    }

    #[test]
    fn test_init() {
        if let Ok(blkdevs) = std::env::var(HAMMER2_BLKDEVS) {
            if let Err(e) = init_std_logger() {
                panic!("{e}");
            }
            let fso = match super::init(&blkdevs, true) {
                Ok(v) => v,
                Err(e) => panic!("{e}"),
            };
            fso.log_volumes();
            assert!(fso.get_nvolumes() > 0);
            assert!(fso.get_nvolumes() <= crate::fs::HAMMER2_MAX_VOLUMES.into());
            assert!(fso.get_total_size() > 0);
            assert_eq!(
                fso.get_total_size() & crate::fs::HAMMER2_VOLUME_ALIGNMASK,
                0
            );

            let Some(vol) = fso.get_root_volume() else {
                panic!("")
            };
            assert_eq!(vol.get_id(), crate::fs::HAMMER2_ROOT_VOLUME.into());
            assert!(std::fs::metadata(vol.get_path()).is_ok());

            assert!(fso.get_volume(fso.get_total_size() - 1).is_some());
            assert!(fso.get_volume(fso.get_total_size()).is_none());

            for i in 0..fso.get_nvolumes() {
                let vol = &fso[i];
                assert_eq!(vol.get_id(), i, "{i}");
                assert!(std::fs::metadata(vol.get_path()).is_ok(), "{i}");
            }
        }
    }
}
