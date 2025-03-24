use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use log::info;

struct SimpleFs;

const TTL: Duration = Duration::from_secs(1);
const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const HELLO_TXT_CONTENT: &str = "This is a new file\n";
const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: HELLO_TXT_CONTENT.len() as u64,
    blocks: 1,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

impl Filesystem for SimpleFs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        info!("lookup(parent={}, name={:?})", parent, name);

        if parent == 1 && name == "fuse.txt" {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        info!("getattr(ino={})", ino);
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        info!("read(ino={}, offset={})", ino, offset);

        if ino == 2 {
            let data = HELLO_TXT_CONTENT.as_bytes();
            if offset as usize >= data.len() {
                reply.data(&[]);
            } else {
                reply.data(&data[offset as usize..]);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        info!("readdir(ino={}, offset={})", ino, offset);

        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "fuse.txt"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}

fn main() {
    env_logger::init();

    let mountpoint = Path::new("/tmp/simple_fuse");

    if !mountpoint.exists() {
        std::fs::create_dir_all(mountpoint).unwrap();
    }

    let options = vec![MountOption::RO, MountOption::FSName("simplefs".to_string())];

    info!("Mounting simple filesystem at {:?}", mountpoint);

    match fuser::mount2(SimpleFs, mountpoint, &options) {
        Ok(()) => info!("Filesystem unmounted"),
        Err(e) => eprintln!("Error mounting filesystem: {}", e),
    }
     
}
