// src/vfs.rs
use core::str;
use crate::println;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Ramdisk,
    ISO9660,
    Disk,
    Partition,
    Network,
    Null,
    Zero,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ramdisk => "ram",
            Self::ISO9660 => "iso",
            Self::Disk => "disk",
            Self::Partition => "part",
            Self::Network => "net",
            Self::Null => "null",
            Self::Zero => "zero",
        }
    }
}

pub trait FileSystem {
    fn mount(&mut self) -> bool;
    fn unmount(&mut self);
    fn read_file(&self, path: &str) -> Option<&'static [u8]>;
    fn list_directory(&self, path: &str, callback: &mut dyn FnMut(&str));
    fn file_exists(&self, path: &str) -> bool;
    fn directory_exists(&self, path: &str) -> bool;
}

pub struct VfsNode {
    pub device_type: DeviceType,
    pub device_index: u32,
    pub mount_point: &'static str,
    pub filesystem: Option<&'static mut dyn FileSystem>,
    pub readonly: bool,
}

pub struct Vfs {
    nodes: [Option<VfsNode>; 16],
    node_count: usize,
}

impl Vfs {
    pub const fn new() -> Self {
        Self {
            nodes: [
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
            node_count: 0,
        }
    }

    pub fn mount(&mut self, device_type: DeviceType, index: u32, mount_point: &'static str, fs: &'static mut dyn FileSystem, readonly: bool) -> bool {
        if self.node_count >= self.nodes.len() {
            println!("VFS: No free node slots");
            return false;
        }

        for node in self.nodes.iter_mut() {
            if node.is_none() {
                if fs.mount() {
                    *node = Some(VfsNode {
                        device_type,
                        device_index: index,
                        mount_point,
                        filesystem: Some(fs),
                        readonly,
                    });
                    self.node_count += 1;
                    println!("VFS: Mounted {}", mount_point);
                    return true;
                }
                return false;
            }
        }
        false
    }

    pub fn find_fs(&self, path: &str) -> Option<(&dyn FileSystem, &'static str)> {
        let mut best_match: Option<(&dyn FileSystem, &'static str)> = None;
        let mut best_len = 0;

        for node in self.nodes.iter() {
            if let Some(n) = node {
                let mp = n.mount_point;
                let matches = if path == mp || path.starts_with(mp) {
                    true
                } else {
                    false
                };

                if matches {
                    let match_len = mp.len();
                    if match_len >= best_len || best_match.is_none() {
                        if let Some(fs) = n.filesystem.as_ref() {
                            best_match = Some((*fs, mp));
                            best_len = match_len;
                        }
                    }
                }
            }
        }
        best_match
    }

    fn strip_mount_point<'a>(&self, path: &'a str, mount_point: &str) -> &'a str {
        if path.starts_with(mount_point) {
            &path[mount_point.len()..]
        } else {
            path
        }
    }

    pub fn read_file(&self, path: &str) -> Option<&'static [u8]> {
        if let Some((fs, mp)) = self.find_fs(path) {
            let rel = self.strip_mount_point(path, mp);
            fs.read_file(rel)
        } else {
            None
        }
    }

    pub fn list_directory(&self, path: &str, callback: &mut dyn FnMut(&str)) {
        if let Some((fs, mp)) = self.find_fs(path) {
            let rel = self.strip_mount_point(path, mp);
            fs.list_directory(rel, callback);
        }
    }

    pub fn file_exists(&self, path: &str) -> bool {
        if let Some((fs, mp)) = self.find_fs(path) {
            let rel = self.strip_mount_point(path, mp);
            fs.file_exists(rel)
        } else {
            false
        }
    }

    pub fn directory_exists(&self, path: &str) -> bool {
        if let Some((fs, mp)) = self.find_fs(path) {
            let rel = self.strip_mount_point(path, mp);
            fs.directory_exists(rel)
        } else {
            false
        }
    }
}

use core::sync::atomic::{AtomicBool, Ordering};

static mut VFS_INSTANCE: Option<Vfs> = None;
static VFS_LOCK: AtomicBool = AtomicBool::new(false);

pub fn init_vfs() {
    unsafe {
        while VFS_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }

        if VFS_INSTANCE.is_none() {
            VFS_INSTANCE = Some(Vfs::new());
            println!("VFS: Initialized");
        }

        VFS_LOCK.store(false, Ordering::Release);
    }
}

pub fn get_vfs() -> Option<&'static mut Vfs> {
    unsafe {
        VFS_INSTANCE.as_mut()
    }
}

// RAM Disk Filesystem Tĩnh
pub struct RamDiskFs {
    mounted: bool,
    files: [(&'static str, &'static [u8]); 16],
    file_count: usize,
    dirs: [&'static str; 8],
    dir_count: usize,
}

impl RamDiskFs {
    pub const fn new() -> Self {
        Self {
            mounted: false,
            files: [("", &[]); 16],
            file_count: 0,
            dirs: [""; 8],
            dir_count: 0,
        }
    }

    pub fn add_file(&mut self, name: &'static str, data: &'static [u8]) -> bool {
        if self.file_count >= 16 { return false; }
        self.files[self.file_count] = (name, data);
        self.file_count += 1;
        true
    }

    pub fn add_dir(&mut self, name: &'static str) -> bool {
        if self.dir_count >= 8 { return false; }
        self.dirs[self.dir_count] = name;
        self.dir_count += 1;
        true
    }
}

impl FileSystem for RamDiskFs {
    fn mount(&mut self) -> bool { self.mounted = true; true }
    fn unmount(&mut self) { self.mounted = false; }

    fn read_file(&self, path: &str) -> Option<&'static [u8]> {
        let normalized = path.trim_matches('/');
        for (name, data) in self.files.iter() {
            if *name == normalized {
                return Some(*data);
            }
        }
        None
    }

    fn list_directory(&self, path: &str, callback: &mut dyn FnMut(&str)) {
        let target = path.trim_matches('/');

        for (name, _) in self.files.iter() {
            if name.is_empty() { continue; }
            if target.is_empty() {
                if !name.contains('/') { callback(name); }
            } else if name.starts_with(target) {
                let rem = &name[target.len()..];
                if rem.starts_with('/') {
                    let entry = rem[1..].split('/').next().unwrap_or("");
                    if !entry.is_empty() { callback(entry); }
                }
            }
        }

        for dir in self.dirs.iter() {
            let clean_dir = dir.trim_matches('/');
            if clean_dir.is_empty() { continue; }
            if target.is_empty() {
                if !clean_dir.contains('/') { callback(clean_dir); }
            } else if clean_dir.starts_with(target) {
                let rem = &clean_dir[target.len()..];
                if rem.starts_with('/') {
                    let entry = rem[1..].split('/').next().unwrap_or("");
                    if !entry.is_empty() { callback(entry); }
                }
            }
        }
    }

    fn file_exists(&self, path: &str) -> bool {
        let normalized = path.trim_matches('/');
        for (name, _) in self.files.iter() {
            if *name == normalized { return true; }
        }
        false
    }

    fn directory_exists(&self, path: &str) -> bool {
        let normalized = path.trim_matches('/');
        if normalized.is_empty() { return true; }
        for dir in self.dirs.iter() {
            if dir.trim_matches('/') == normalized { return true; }
        }
        false
    }
}