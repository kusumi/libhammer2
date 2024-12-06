use crate::fs;
use crate::os;

pub const HAMMER2IOC: u8 = b'h';

pub const HAMMER2IOC_VERSION_GET: u8 = 64;
pub const HAMMER2IOC_PFS_GET: u8 = 80;
pub const HAMMER2IOC_PFS_CREATE: u8 = 81;
pub const HAMMER2IOC_PFS_DELETE: u8 = 82;
pub const HAMMER2IOC_PFS_LOOKUP: u8 = 83;
pub const HAMMER2IOC_PFS_SNAPSHOT: u8 = 84;
pub const HAMMER2IOC_INODE_GET: u8 = 86;
pub const HAMMER2IOC_INODE_SET: u8 = 87;
pub const HAMMER2IOC_DEBUG_DUMP: u8 = 91;
pub const HAMMER2IOC_BULKFREE_SCAN: u8 = 92;
pub const HAMMER2IOC_DESTROY: u8 = 94;
pub const HAMMER2IOC_EMERG_MODE: u8 = 95;
pub const HAMMER2IOC_GROWFS: u8 = 96;
pub const HAMMER2IOC_VOLUME_LIST: u8 = 97;

#[repr(C)]
#[derive(Debug)]
pub struct Hammer2IocVersion {
    pub version: u32,
    pub reserved: [u8; 252],
}

impl Default for Hammer2IocVersion {
    fn default() -> Self {
        Self::new()
    }
}

impl Hammer2IocVersion {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: 0,
            reserved: [0; 252],
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Hammer2IocPfs {
    pub name_key: u64,  // super-root directory scan
    pub name_next: u64, // (GET only)
    pub pfs_type: u8,
    pub pfs_subtype: u8,
    pub reserved0012: u8,
    pub reserved0013: u8,
    pub pfs_flags: u32,
    pub reserved0018: u64,
    pub pfs_fsid: [u8; 16], // identifies PFS instance
    pub pfs_clid: [u8; 16], // identifies PFS cluster
    pub name: [u8; os::NAME_MAX + 1],
}

impl Default for Hammer2IocPfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Hammer2IocPfs {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name_key: 0,
            name_next: 0,
            pfs_type: 0,
            pfs_subtype: 0,
            reserved0012: 0,
            reserved0013: 0,
            pfs_flags: 0,
            reserved0018: 0,
            pfs_fsid: [0; 16],
            pfs_clid: [0; 16],
            name: [0; os::NAME_MAX + 1],
        }
    }
}

pub const HAMMER2_PFSFLAGS_NOSYNC: u32 = 0x0000_0001;

#[repr(C)]
#[derive(Debug, Default)]
pub struct Hammer2IocInode {
    pub flags: u32,
    pub unused: u64, // XXX void* in DragonFly
    pub data_count: u64,
    pub inode_count: u64,
    pub ip_data: fs::Hammer2InodeData,
}

impl Hammer2IocInode {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

pub const HAMMER2IOC_INODE_FLAG_IQUOTA: u32 = 0x0000_0001;
pub const HAMMER2IOC_INODE_FLAG_DQUOTA: u32 = 0x0000_0002;
pub const HAMMER2IOC_INODE_FLAG_COPIES: u32 = 0x0000_0004;
pub const HAMMER2IOC_INODE_FLAG_CHECK: u32 = 0x0000_0008;
pub const HAMMER2IOC_INODE_FLAG_COMP: u32 = 0x0000_0010;

#[repr(C)]
#[derive(Debug, Default)]
pub struct Hammer2IocBulkfree {
    pub sbase: u64,            // starting storage offset
    pub sstop: u64,            // (set on return)
    pub size: u64,             // swapable kernel memory to use; XXX size_t in DragonFly
    pub count_allocated: u64,  // alloc fixups this run
    pub count_freed: u64,      // bytes freed this run
    pub total_fragmented: u64, // merged result
    pub total_allocated: u64,  // merged result
    pub total_scanned: u64,    // bytes of storage
}

impl Hammer2IocBulkfree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Hammer2IocDestroy {
    pub cmd: u32, // XXX enum in DragonFly
    pub path: [u8; fs::HAMMER2_INODE_MAXNAME],
    pub inum: u64,
}

impl Default for Hammer2IocDestroy {
    fn default() -> Self {
        Self::new()
    }
}

impl Hammer2IocDestroy {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cmd: HAMMER2_DELETE_NOP,
            path: [0; fs::HAMMER2_INODE_MAXNAME],
            inum: 0,
        }
    }
}

pub const HAMMER2_DELETE_NOP: u32 = 0;
pub const HAMMER2_DELETE_FILE: u32 = 1;
pub const HAMMER2_DELETE_INUM: u32 = 2;

#[repr(C)]
#[derive(Debug, Default)]
pub struct Hammer2IocGrowfs {
    pub size: u64,
    pub modified: u32,
    pub unused01: u32,
    pub unusedary: [u32; 14],
}

impl Hammer2IocGrowfs {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Hammer2IocVolume {
    pub path: [u8; os::MAXPATHLEN],
    pub id: u32,
    pub offset: u64,
    pub size: u64,
}

impl Default for Hammer2IocVolume {
    fn default() -> Self {
        Self::new()
    }
}

impl Hammer2IocVolume {
    #[must_use]
    pub fn new() -> Self {
        Self {
            path: [0; os::MAXPATHLEN],
            id: 0,
            offset: 0,
            size: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Hammer2IocVolumeList {
    pub volumes: u64, // XXX hammer2_ioc_volume_t* in DragonFly
    pub nvolumes: u32,
    pub version: u32,
    pub pfs_name: [u8; fs::HAMMER2_INODE_MAXNAME],
}

impl Default for Hammer2IocVolumeList {
    fn default() -> Self {
        Self::new()
    }
}

impl Hammer2IocVolumeList {
    #[must_use]
    pub fn new() -> Self {
        Self {
            volumes: 0,
            nvolumes: 0,
            version: 0,
            pfs_name: [0; fs::HAMMER2_INODE_MAXNAME],
        }
    }
}
