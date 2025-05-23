// Copyright (C) 2023-2024  DCsunset

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
  ReplyDirectory,
  FUSE_ROOT_ID
};
use std::{
  fs,
  iter,
  os::unix::prelude::FileExt,
  time::{Duration, SystemTime}, ffi::{OsString, OsStr}, io,
  collections::HashMap,
  path::{Path, PathBuf}, cmp
};
use log::{error, warn};
use crate::metadata::{InodeInfo, InodeConfig};
use libc::{EIO, ENOENT};
use ipc_channel::ipc;

pub struct RangeFs {
  file: PathBuf,
  /// Timeout for cache in fuse reply (attr, entry)
  timeout: Duration,
  /// Map file name to inode
  file_map: HashMap<OsString, u64>,
  /// map inode to actual filename and metadata
  inode_map: HashMap<u64, InodeInfo>,
  /// Channel sender to send signal on init
  init_tx: Option<ipc::IpcSender<Option<String>>>,
}

impl Default for InodeConfig {
  fn default() -> Self {
    Self {
      name: None,
      offset: None,
      size: None,
      uid: None,
      gid: None,
      preload: false,
    }
  }
}

impl RangeFs {
  pub fn new(file: PathBuf, configs: Vec<InodeConfig>, timeout_secs: u64, init_tx: Option<ipc::IpcSender<Option<String>>>) -> Self {
    let (file_map, inode_map) = RangeFs::init_file_inode_map(&file, configs);
    Self {
      file,
      timeout: Duration::from_secs(timeout_secs),
      file_map,
      inode_map,
      init_tx,
    }
  }

  /// Init file_map and inode_map
  fn init_file_inode_map(file: impl AsRef<Path>, configs: Vec<InodeConfig>) -> (HashMap<OsString, u64>, HashMap<u64, InodeInfo>) {
    let mut file_map: HashMap<OsString, _> = HashMap::new();
    let mut inode_map = HashMap::new();

    // ino start fro 2 as 1 is reserved for FUSE root directory
    for (ino, config) in iter::zip(2.., configs) {
      // use original device name as default name if not specified
      // let name = n.unwrap_or(path).as_os_str().to_os_string();
      let name: OsString = match &config.name {
        Some(name) => name.into(),
        None => {
          file.as_ref().file_name()
            .expect(&format!("invalid source file: {:?}", file.as_ref()))
            .into()
        }
      };
      match file_map.get(&name) {
        Some(_) => warn!("Ignoring config with duplicate name: {:?}", name),
        None => {
          let preload = config.preload;
          let mut info = InodeInfo::new(&file, ino, config);
          if preload {
            let data = Self::read_file(&file, &info, 0, info.attr.size);
            // error preloading
            if data.is_none() {
              continue;
            }
            info.data = data
          }
          inode_map.insert(ino, info);
          file_map.insert(name, ino);
        }
      };
    }
    (file_map, inode_map)
  }

  /// Read a virtual file data
  fn read_file(file: impl AsRef<Path>, info: &InodeInfo, offset: u64, size: u64) -> Option<Vec<u8>> {
    if info.err {
      return None;
    }
    let o = info.config.offset.unwrap_or(0) + offset as u64;
    let s = cmp::min(info.attr.size.saturating_sub(offset as u64), size as u64);
    match read_at(&file, o, s as usize) {
      Ok(data) => Some(data),
      Err(err) => {
        error!("Error reading file {:?}: {}", file.as_ref(), err);
        None
      }
    }
  }

}


impl Filesystem for RangeFs {
  // Invoked after file system is mounted
  fn init(&mut self, _req: &Request<'_>, _config: &mut fuser::KernelConfig) -> Result<(), libc::c_int> {
    if let Some(tx) = &self.init_tx {
      tx.send(None).unwrap()
    }
    Ok(())
  }

  fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: fuser::ReplyEntry) {
    // Only one root directory
    if parent != FUSE_ROOT_ID {
      reply.error(ENOENT);
      return;
    }
    match self.file_map.get(name) {
      Some(ino) => {
        let info = self.inode_map.get_mut(ino).expect(&format!("invalid ino: {}", ino));
        info.update_info(&self.file, self.timeout);
        reply.entry(&self.timeout, &info.attr, 0);
      },
      None => {
        reply.error(ENOENT);
      }
    };
  }

  fn getattr(&mut self, _req: &Request, ino: u64, reply: fuser::ReplyAttr) {
    if ino == FUSE_ROOT_ID {
      let cur_time = SystemTime::now();
      reply.attr(&self.timeout, &fuser::FileAttr {
        ino: FUSE_ROOT_ID,
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
        info.update_info(&self.file, self.timeout);
        if info.err {
          reply.error(EIO);
          return;
        }
        reply.attr(&self.timeout, &info.attr);
      } else {
        reply.error(ENOENT);
      }
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
    if ino != FUSE_ROOT_ID {
      reply.error(ENOENT);
      return;
    }
    assert!(offset >= 0);

    let entries = self.file_map.iter().map(|(name, ino)| {
      (ino.clone(), FileType::RegularFile, name.to_os_string())
    });

    for (i, e) in entries.enumerate().skip(offset as usize) {
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
        info.update_info(&self.file, self.timeout);
        if info.err {
          reply.error(EIO);
          return;
        }
        // Return dummy fh and flags as we only use ino in read
        reply.opened(0, 0);
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
        if info.err {
          reply.error(EIO);
          return;
        }
        if let Some(data) = &info.data {
          let s = cmp::min(offset as usize, data.len());
          let e = cmp::min(s + size as usize, data.len());
          reply.data(&data[s..e]);
          return;
        }

        match Self::read_file(&self.file, info, offset as u64, size as u64) {
          Some(data) => reply.data(&data),
          None => reply.error(EIO),
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
