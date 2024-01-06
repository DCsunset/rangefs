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

use std::{time::{SystemTime, Duration}, fs, os::unix::prelude::MetadataExt, path::Path};

use fuser::{FileAttr, FileType};
use libc::{S_IXUSR, S_IXGRP, S_IXOTH, S_IFMT};
use log::{warn, debug};

/// Config for each mapped file
pub struct InodeConfig {
  pub name: Option<String>,
  pub offset: Option<u64>,
  pub size: Option<u64>,
  pub uid: Option<u32>,
  pub gid: Option<u32>,
}

// InodeInfo corresponds to top level dirs
pub struct InodeInfo {
  pub ino: u64,
  // whether error encountered when reading source file metadata
  pub err: bool,
  /// Actual attr of the virtual file
  pub attr: FileAttr,
  pub config: InodeConfig,
  /// Last update timestamp
  timestamp: SystemTime
}

impl InodeInfo {
  pub fn new(file: impl AsRef<Path>, ino: u64, config: InodeConfig) -> Self {
    let (attr, err) = InodeInfo::get_metadata(file, ino, &config);
    Self {
      ino,
      err,
      attr,
      config,
      timestamp: SystemTime::now()
    }
  }

  pub fn outdated(&self, now: SystemTime, timeout: Duration) -> bool {
    match now.duration_since(self.timestamp) {
      Ok(elapsed) => {
        // update if outdated
        elapsed > timeout
      },
      Err(err) => {
        warn!("System time error: {err}");
        // Always outdated since timestamp should be before now
        true
      }
    }
  }

  pub fn update_info(&mut self, file: impl AsRef<Path>, timeout: Duration) {
    if self.outdated(SystemTime::now(), timeout) {
      debug!("Updating inode info");
      let (attr, err) = InodeInfo::get_metadata(file, self.ino, &self.config);
      self.attr = attr;
      self.err = err;
      self.timestamp = SystemTime::now();
    }
  }

  // Get and derive attr from metadata of existing file
  pub fn get_metadata(file: impl AsRef<Path>, ino: u64, config: &InodeConfig) -> (FileAttr, bool) {
    let cur_time = SystemTime::now();
    match fs::metadata(file) {
      Ok(src_metadata) => {
        // permission bits (excluding the format bits)
        let mut perm = src_metadata.mode() & !S_IFMT;
        if src_metadata.is_dir() {
          // remove executable bit
          perm &= !(S_IXUSR | S_IXGRP | S_IXOTH);
        }
        let size = config.size.unwrap_or(src_metadata.size().saturating_sub(config.offset.unwrap_or(0)));

        (FileAttr {
          ino,
          size,
          blocks: size.div_ceil(512),
          // Convert unix timestamp to SystemTime
          atime: src_metadata.accessed().unwrap_or(cur_time),
          mtime: src_metadata.modified().unwrap_or(cur_time),
          ctime: src_metadata.accessed().unwrap_or(cur_time),
          crtime: src_metadata.created().unwrap_or(cur_time), // macOS only
          kind: FileType::RegularFile,
          perm: perm as u16,
          nlink: 1,
          uid: config.uid.unwrap_or(src_metadata.uid()),
          gid: config.gid.unwrap_or(src_metadata.gid()),
          rdev: 0,
          blksize: 512,
          flags: 0 // macOS only
        }, false)
      }
      Err(err) => {
        warn!("Error reading source file metadata: {}", err);
        let size = config.size.unwrap_or(0);
        // dummy attr
        (FileAttr {
          ino,
          size: 0,
          blocks: size.div_ceil(512),
          // Convert unix timestamp to SystemTime
          atime: cur_time,
          mtime: cur_time,
          ctime: cur_time,
          crtime: cur_time, // macOS only
          kind: FileType::RegularFile,
          perm: 0o666,
          nlink: 1,
          uid: config.uid.unwrap_or(0),
          gid: config.gid.unwrap_or(0),
          rdev: 0,
          blksize: 512,
          flags: 0 // macOS only
        }, true)
      }
    }
  }
}
