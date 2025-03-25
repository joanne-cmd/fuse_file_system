use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyCreate, ReplyData, 
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyWrite, Request,
};
use libc::{ENOENT, EISDIR, EEXIST};
use log::{info, warn, error};

#[derive(Clone)]
struct FileEntry {
    attr: FileAttr,
    content: Vec<u8>,
}

struct SimpleFs {
    files: Arc<Mutex<HashMap<u64, FileEntry>>>,
    next_inode: Arc<Mutex<u64>>,
}

impl SimpleFs {
    fn new() -> Self {
        let mut files = HashMap::new();
        
        
        files.insert(1, FileEntry {
            attr: HELLO_DIR_ATTR,
            content: Vec::new(),
        });

        
        files.insert(2, FileEntry {
            attr: HELLO_TXT_ATTR,
            content: HELLO_TXT_CONTENT.as_bytes().to_vec(),
        });

        SimpleFs {
            files: Arc::new(Mutex::new(files)),
            next_inode: Arc::new(Mutex::new(3)), 
        }
    }

    fn get_current_time() -> (SystemTime, SystemTime) {
        let now = SystemTime::now();
        (now, now)
    }
}

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
    size: 17,
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
        
        let files = self.files.lock().unwrap();
        
        if parent != 1 {
            warn!("Lookup failed: parent {} is not a directory", parent);
            reply.error(ENOENT);
            return;
        }

      
        for (ino, entry) in files.iter() {
            if entry.attr.kind != FileType::Directory && 
               OsStr::new(name.to_str().unwrap_or("")) == OsStr::new(name.to_str().unwrap_or("")) {
                reply.entry(&TTL, &entry.attr, 0);
                return;
            }
        }

        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        info!("getattr(ino={})", ino);
        
        let files = self.files.lock().unwrap();
        
        match files.get(&ino) {
            Some(entry) => reply.attr(&TTL, &entry.attr),
            None => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        info!("read(ino={}, offset={}, size={})", ino, offset, size);
        
        let files = self.files.lock().unwrap();
        
        match files.get(&ino) {
            Some(entry) => {
                if entry.attr.kind == FileType::Directory {
                    reply.error(EISDIR);
                    return;
                }
                
                let data = &entry.content;
                if offset as usize >= data.len() {
                    reply.data(&[]);
                } else {
                    let end = std::cmp::min(offset as usize + size as usize, data.len());
                    reply.data(&data[offset as usize..end]);
                }
            },
            None => reply.error(ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        info!("readdir(ino={}, offset={})", ino, offset);
        
        let _files = self.files.lock().unwrap();
        
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

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        info!("create(parent={}, name={:?}, mode={})", parent, name, mode);
        
        let mut files = self.files.lock().unwrap();
        let mut next_inode = self.next_inode.lock().unwrap();
        
        
        if parent != 1 {
            warn!("Create failed: parent {} is not a directory", parent);
            reply.error(ENOENT);
            return;
        }

    
        let name_str = name.to_str().unwrap_or("");
        if files.values().any(|entry| 
            entry.attr.kind != FileType::Directory && 
            entry.attr.ino != 1 && 
            entry.attr.ino != 2
        ) {
            warn!("Create failed: file {} already exists", name_str);
            reply.error(EEXIST);
            return;
        }

        let (now, now2) = Self::get_current_time();
        let new_inode = *next_inode;
        *next_inode += 1;

        let file_attr = FileAttr {
            ino: new_inode,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now2,
            ctime: now2,
            crtime: now2,
            kind: FileType::RegularFile,
            perm: (mode & 0o777) as u16,
            nlink: 1,
            uid: _req.uid(),
            gid: _req.gid(),
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        files.insert(new_inode, FileEntry {
            attr: file_attr,
            content: Vec::new(),
        });

        reply.created(&TTL, &file_attr, 0, 0, 0);
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        info!("write(ino={}, offset={}, data_len={})", ino, offset, data.len());
        
        let mut files = self.files.lock().unwrap();
        
        match files.get_mut(&ino) {
            Some(entry) => {
                if entry.attr.kind == FileType::Directory {
                    warn!("Write failed: cannot write to a directory");
                    reply.error(EISDIR);
                    return;
                }

               
                let start = offset as usize;
                if start > entry.content.len() {
                    entry.content.resize(start, 0);
                }
                entry.content.splice(start..start, data.iter().cloned());

                
                let (_, now2) = Self::get_current_time();
                entry.attr.size = entry.content.len() as u64;
                entry.attr.mtime = now2;
                entry.attr.ctime = now2;

                reply.written(data.len() as u32);
            },
            None => {
                warn!("Write failed: inode {} not found", ino);
                reply.error(ENOENT);
            }
        }
    }

    fn unlink(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        info!("unlink(parent={}, name={:?})", parent, name);
        
        let mut files = self.files.lock().unwrap();
        
        
        if parent != 1 {
            warn!("Unlink failed: parent {} is not a directory", parent);
            reply.error(ENOENT);
            return;
        }

        let name_str = name.to_str().unwrap_or("");
        
        
        let file_to_delete = files.iter()
            .find(|(_, entry)| 
                entry.attr.kind != FileType::Directory && 
                entry.attr.ino != 1
            )
            .map(|(ino, _)| *ino);

        match file_to_delete {
            Some(ino) => {
                files.remove(&ino);
                reply.ok();
            },
            None => {
                warn!("Unlink failed: file {} not found", name_str);
                reply.error(ENOENT);
            }
        }
    }
}

fn main() {
    env_logger::init();
    let mountpoint = Path::new("/tmp/simple_fuse");
    if !mountpoint.exists() {
        std::fs::create_dir_all(mountpoint).unwrap();
    }
    let options = vec![MountOption::RW, MountOption::FSName("simplefs".to_string())];
    
    info!("Mounting simple filesystem at {:?}", mountpoint);
    match fuser::mount2(SimpleFs::new(), mountpoint, &options) {
        Ok(()) => info!("Filesystem unmounted"),
        Err(e) => error!("Error mounting filesystem: {}", e),
    }
}