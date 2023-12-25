// Copyright (C) 2023  DCsunset

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use fuser::{
  Filesystem,
  FileType,
  Request,
  ReplyDirectory
};
use std::{
  fs,
  iter,
  os::unix::prelude::FileExt,
  time::{Duration, SystemTime}, ffi::{OsString, OsStr}, io,
  collections::HashMap,
  path::{Path, PathBuf}, cmp
};
use log::{error, warn, info};
use crate::metadata::{InodeInfo, ROOT_INODE, InodeInfoOptions};
use libc::{EIO, ENOENT};
use itertools::izip;

pub struct RangeFs {
  /// Timeout for cache in fuse reply (attr, entry)
  timeout: Duration,
  // Map file name to inode
  file_map: HashMap<OsString, u64>,
  /// map inode to actual filename and metadata
  inode_map: HashMap<u64, InodeInfo>
}

impl RangeFs {
  pub fn new(files: Vec<PathBuf>, offsets: Vec<u64>, sizes: Vec<u64>, names: Vec<PathBuf>, timeout_secs: u64) -> Self {
    let (file_map, inode_map) = RangeFs::init_file_inode_map(&files, &offsets, &sizes, &names);
    Self {
      timeout: Duration::from_secs(timeout_secs),
      file_map,
      inode_map
    }
  }

  /// Init file_map and inode_map
  fn init_file_inode_map(paths: &Vec<PathBuf>, offsets: &Vec<u64>, sizes: &Vec<u64>, names: &Vec<PathBuf>) -> (HashMap<OsString, u64>, HashMap<u64, InodeInfo>) {
    let mut file_map: HashMap<OsString, _> = HashMap::new();
    let mut inode_map = HashMap::new();
    for (ino, path, offset, size, n) in izip!(
      // ino start fro 2 as 1 is for ROOT_INODE
      2..,
      paths,
      // default offset is 0
      offsets.iter().cloned().chain(iter::repeat(0)),
      sizes.iter().map(|s| Some(s.clone())).chain(iter::repeat(None)),
      names.iter().map(|n| Some(n)).chain(iter::repeat(None))
    ) {
      // use original device name as default name if not specified
      // let name = n.unwrap_or(path).as_os_str().to_os_string();
      let name: OsString = match n {
        Some(name) => name.into(),
        None => {
          path.as_path().file_name()
            .expect(&format!("invalid source file: {:?}", path))
            .into()
        }
      };
      match file_map.get(&name) {
        Some(_) => info!("duplicate source paths"),
        None => {
          match InodeInfo::new(InodeInfoOptions {
            path: path.into(),
            ino,
            offset,
            size
          }) {
            Ok(info) => {
              file_map.insert(name, ino);
              inode_map.insert(ino, info);
            }
            Err(err) => {
              warn!("error creating inode info: {}", err);
            }
          }
        }
      };
    }
    (file_map, inode_map)
  }
}


impl Filesystem for RangeFs {
  fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: fuser::ReplyEntry) {
    // Only one root directory
    if parent != ROOT_INODE {
      reply.error(ENOENT);
      return;
    }
    match self.file_map.get(name) {
      Some(ino) => {
        let info = self.inode_map.get_mut(ino).expect(&format!("invalid ino: {}", ino));
        match info.update_info(self.timeout) {
          Ok(_) => {
            reply.entry(&self.timeout, &info.attr, 0);
            return;
          }
          Err(err) => {
            warn!("error updating inode info: {}", err);
            self.inode_map.remove(ino);
            if let Some(e) = err.raw_os_error() {
              reply.error(e);
              return;
            }
          }
        };
      },
      None => {
        reply.error(ENOENT);
      }
    };
  }

  fn getattr(&mut self, _req: &Request, ino: u64, reply: fuser::ReplyAttr) {
    if ino == ROOT_INODE {
      let cur_time = SystemTime::now();
      reply.attr(&self.timeout, &fuser::FileAttr {
        ino: ROOT_INODE,
        size: 0,
        blocks: 0,
        atime: cur_time,
        mtime: cur_time,
        ctime: cur_time,
        crtime: cur_time,
        kind: FileType::Directory,
        perm: 0o777,
        nlink: 1,
        uid: 0,
        gid: 0,
        rdev: 0,
        blksize: 512,
        flags: 0
      });
    } else {
      if let Some(info) = self.inode_map.get_mut(&ino) {
        match info.update_info(self.timeout) {
          Ok(_) => {
            reply.attr(&self.timeout, &info.attr);
            return;
          },
          Err(err) => {
            warn!("error updating inode info: {}", err);
          }
        }
      }
      reply.error(ENOENT);
    }
  }

  fn readdir(
    &mut self,
    _req: &Request,
    ino: u64,
    _fh: u64,  // use inode only as we returned a dummy fh for opendir (by default 0)
    offset: i64,
    mut reply: ReplyDirectory,
  ) {
    if ino != ROOT_INODE {
      reply.error(ENOENT);
      return;
    }
    assert!(offset >= 0);

    // special entries
    let mut entries = vec![
      (ROOT_INODE, FileType::Directory, OsString::from(".")),
      (ROOT_INODE, FileType::Directory, OsString::from("..")),
    ];
    entries.extend(self.file_map.iter().map(|(name, ino)| {
      (ino.clone(), FileType::RegularFile, name.to_os_string())
    }));

    for (i, e) in entries.iter().enumerate().skip(offset as usize) {
      // offset is used by kernel for future readdir calls (should be next entry)
      if reply.add(e.0, (i+1) as i64, e.1, &e.2) {
        // return true when buffer full
        break;
      }
    }

    reply.ok();
  }

  fn open(&mut self, _req: &Request, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
    match self.inode_map.get_mut(&ino) {
      Some(info) => {
        match info.update_info(self.timeout) {
          Ok(_) => reply.opened(0, 0),
          Err(err) => {
            warn!("error opening file {:?}: {}", info.options.path, err);
            if let Some(e) = err.raw_os_error() {
              reply.error(e);
              return;
            }
          }
        }
        // Return dummy fh and flags as we only use ino in read
      },
      None => reply.error(ENOENT)
    };
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
    reply: fuser::ReplyData,
  ) {
    assert!(offset >= 0);
    match self.inode_map.get(&ino) {
      Some(info) => {
        let o = info.options.offset + offset as u64;
        let s = cmp::min(info.attr.size.saturating_sub(offset as u64), size as u64);
        match read_at(&info.options.path, o, s as usize) {
          Ok(data) => {
            reply.data(&data);
          },
          Err(err) => {
            error!("error reading file {:?}: {}", info.options.path, err);
            reply.error(EIO);
          }
        }
      },
      None => reply.error(ENOENT)
    };
  }

  fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: fuser::ReplyStatfs) {
    // Sum up all the blocks
    let blocks: u64 = self.inode_map.values().map(|v| v.attr.blocks).sum();
    // convert to c-style string without encoding/decoding
    reply.statfs(blocks, 0, 0, self.inode_map.len() as u64, 0, 512, 255, 512);
  }
}

fn read_at(path: impl AsRef<Path>, offset: u64, size: usize) -> io::Result<Vec<u8>> {
  let f = fs::File::open(path)?;
  let mut buf = vec![0; size as usize];
  let num = f.read_at(&mut buf, offset)?;
  buf.resize(num, 0);
  Ok(buf)
}
