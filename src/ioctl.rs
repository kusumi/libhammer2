const IOC: u8 = b'h';

const IOC_VERSION_GET: u8 = 64;
const IOC_PFS_GET: u8 = 80;
const IOC_PFS_CREATE: u8 = 81;
const IOC_PFS_DELETE: u8 = 82;
const IOC_PFS_LOOKUP: u8 = 83;
const IOC_PFS_SNAPSHOT: u8 = 84;
const IOC_INODE_GET: u8 = 86;
const IOC_INODE_SET: u8 = 87;
const IOC_DEBUG_DUMP: u8 = 91;
const IOC_BULKFREE_SCAN: u8 = 92;
const IOC_DESTROY: u8 = 94;
const IOC_EMERG_MODE: u8 = 95;
const IOC_GROWFS: u8 = 96;
const IOC_VOLUME_LIST: u8 = 97;

// IOC_VERSION_GET
#[repr(C)]
#[derive(Debug)]
pub struct IocVersion {
    pub version: u32,
    pub reserved: [u8; 252],
}

impl Default for IocVersion {
    fn default() -> Self {
        Self::new()
    }
}

impl IocVersion {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: 0,
            reserved: [0; 252],
        }
    }
}

nix::ioctl_readwrite!(version_get, IOC, IOC_VERSION_GET, IocVersion);

// IOC_PFS_XXX
#[repr(C)]
#[derive(Debug)]
pub struct IocPfs {
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
    pub name: [u8; crate::os::NAME_MAX + 1],
}

impl Default for IocPfs {
    fn default() -> Self {
        Self::new()
    }
}

impl IocPfs {
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
            name: [0; crate::os::NAME_MAX + 1],
        }
    }
}

nix::ioctl_readwrite!(pfs_get, IOC, IOC_PFS_GET, IocPfs);
nix::ioctl_readwrite!(pfs_create, IOC, IOC_PFS_CREATE, IocPfs);
nix::ioctl_readwrite!(pfs_delete, IOC, IOC_PFS_DELETE, IocPfs);
nix::ioctl_readwrite!(pfs_lookup, IOC, IOC_PFS_LOOKUP, IocPfs);
nix::ioctl_readwrite!(pfs_snapshot, IOC, IOC_PFS_SNAPSHOT, IocPfs);

pub const PFS_FLAGS_NOSYNC: u32 = 0x0000_0001;

// IOC_INODE_XXX
#[repr(C)]
#[derive(Debug, Default)]
pub struct IocInode {
    pub flags: u32,
    pub unused: u64, // XXX void* in DragonFly
    pub data_count: u64,
    pub inode_count: u64,
    pub ip_data: crate::fs::Hammer2InodeData,
}

impl IocInode {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

nix::ioctl_readwrite!(inode_get, IOC, IOC_INODE_GET, IocInode);
nix::ioctl_readwrite!(inode_set, IOC, IOC_INODE_SET, IocInode);

pub const INODE_FLAGS_IQUOTA: u32 = 0x0000_0001;
pub const INODE_FLAGS_DQUOTA: u32 = 0x0000_0002;
pub const INODE_FLAGS_COPIES: u32 = 0x0000_0004;
pub const INODE_FLAGS_CHECK: u32 = 0x0000_0008;
pub const INODE_FLAGS_COMP: u32 = 0x0000_0010;

// IOC_DEBUG_DUMP
nix::ioctl_readwrite!(debug_dump, IOC, IOC_DEBUG_DUMP, u32);

// IOC_BULKFREE_SCAN
#[repr(C)]
#[derive(Debug, Default)]
pub struct IocBulkfree {
    pub sbase: u64,            // starting storage offset
    pub sstop: u64,            // (set on return)
    pub size: u64,             // swapable kernel memory to use; XXX size_t in DragonFly
    pub count_allocated: u64,  // alloc fixups this run
    pub count_freed: u64,      // bytes freed this run
    pub total_fragmented: u64, // merged result
    pub total_allocated: u64,  // merged result
    pub total_scanned: u64,    // bytes of storage
}

impl IocBulkfree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

nix::ioctl_readwrite!(bulkfree_scan, IOC, IOC_BULKFREE_SCAN, IocBulkfree);

// IOC_DESTROY
#[repr(C)]
#[derive(Debug)]
pub struct IocDestroy {
    pub cmd: u32, // XXX enum in DragonFly
    pub path: [u8; crate::fs::HAMMER2_INODE_MAXNAME],
    pub inum: u64,
}

impl Default for IocDestroy {
    fn default() -> Self {
        Self::new()
    }
}

impl IocDestroy {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cmd: DESTROY_CMD_NOP,
            path: [0; crate::fs::HAMMER2_INODE_MAXNAME],
            inum: 0,
        }
    }
}

nix::ioctl_readwrite!(destroy, IOC, IOC_DESTROY, IocDestroy);

pub const DESTROY_CMD_NOP: u32 = 0;
pub const DESTROY_CMD_FILE: u32 = 1;
pub const DESTROY_CMD_INUM: u32 = 2;

// IOC_EMERG_MODE
nix::ioctl_readwrite!(emerg_mode, IOC, IOC_EMERG_MODE, u32);

// IOC_GROWFS
#[repr(C)]
#[derive(Debug, Default)]
pub struct IocGrowfs {
    pub size: u64,
    pub modified: u32,
    pub unused01: u32,
    pub unusedary: [u32; 14],
}

impl IocGrowfs {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

nix::ioctl_readwrite!(growfs, IOC, IOC_GROWFS, IocGrowfs);

// IOC_VOLUME_LIST
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IocVolume {
    pub path: [u8; crate::os::MAXPATHLEN],
    pub id: u32,
    pub offset: u64,
    pub size: u64,
}

impl Default for IocVolume {
    fn default() -> Self {
        Self::new()
    }
}

impl IocVolume {
    #[must_use]
    pub fn new() -> Self {
        Self {
            path: [0; crate::os::MAXPATHLEN],
            id: 0,
            offset: 0,
            size: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct IocVolumeList {
    pub volumes: u64, // XXX hammer2_ioc_volume_t* in DragonFly
    pub nvolumes: u32,
    pub version: u32,
    pub pfs_name: [u8; crate::fs::HAMMER2_INODE_MAXNAME],
}

impl Default for IocVolumeList {
    fn default() -> Self {
        Self::new()
    }
}

impl IocVolumeList {
    #[must_use]
    pub fn new() -> Self {
        Self {
            volumes: 0,
            nvolumes: 0,
            version: 0,
            pfs_name: [0; crate::fs::HAMMER2_INODE_MAXNAME],
        }
    }
}

nix::ioctl_readwrite!(volume_list, IOC, IOC_VOLUME_LIST, IocVolumeList);
