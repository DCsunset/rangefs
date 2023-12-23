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

use std::{ffi::OsString, time::{SystemTime, Duration}, fs, os::unix::prelude::MetadataExt, path::Path, io};

use fuser::{FileAttr, FileType};
use libc::{S_IXUSR, S_IXGRP, S_IXOTH, S_IFMT};
use log::warn;

// InodeInfo corresponds to top level dirs
pub struct InodeInfo {
  pub ino: u64,
	pub path: OsString,
	pub attr: FileAttr,
  pub offset: u64,
  pub size: u64,
	/// Last update timestamp
	timestamp: SystemTime
}

impl InodeInfo {
	pub fn new(path: impl AsRef<Path>, ino: u64, offset: u64, size: u64) -> io::Result<Self> {
		let attr = InodeInfo::get_metadata(path.as_ref(), ino, size)?;
		Ok(Self {
			path: path.as_ref().as_os_str().to_os_string(),
      ino,
      offset,
      size,
			attr,
			timestamp: SystemTime::now()
		})
	}

	pub fn outdated(&self, now: SystemTime, timeout: Duration) -> bool {
		match now.duration_since(self.timestamp) {
			Ok(elapsed)	=> {
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

	pub fn update_info(&mut self, timeout: Duration) -> io::Result<()> {
		if self.outdated(SystemTime::now(), timeout) {
			let attr = InodeInfo::get_metadata(&self.path, self.ino, self.size)?;
			self.attr = attr;
			self.timestamp = SystemTime::now();
		}
		Ok(())
	}

	fn get_metadata(path: impl AsRef<Path>, ino: u64, size: u64) -> io::Result<FileAttr> {
		let src_metadata = fs::metadata(&path)?;
		let attr = derive_attr(&src_metadata,	ino, size);
		Ok(attr)
	}
}

// For root, inode must be 1, as specified in https://github.com/libfuse/libfuse/blob/master/include/fuse_lowlevel.h (FUSE_ROOT_ID)
pub const ROOT_INODE: u64 = 1;

// Derive attr from metadata of existing file
pub fn derive_attr(src_metadata: &fs::Metadata, ino: u64, size: u64) -> FileAttr {
	let cur_time = SystemTime::now();
	// permission bits (excluding the format bits)
	let mut perm = src_metadata.mode() & !S_IFMT;
	if src_metadata.is_dir() {
		// remove executable bit
		perm &= !(S_IXUSR | S_IXGRP | S_IXOTH);
	}

	FileAttr {
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
		uid: src_metadata.uid(),
		gid: src_metadata.gid(),
		rdev: 0,
		blksize: 512,
		flags: 0 // macOS only
	}
}

