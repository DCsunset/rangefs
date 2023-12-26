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

use std::{time::{SystemTime, Duration}, fs, os::unix::prelude::MetadataExt, io, ffi::OsString};

use fuser::{FileAttr, FileType};
use libc::{S_IXUSR, S_IXGRP, S_IXOTH, S_IFMT};
use log::warn;

/// Option to init or update InodeInfo
pub struct InodeInfoOptions {
  /// path of the source file
	pub path: OsString,
  pub ino: u64,
  pub start: u64,
  pub length: Option<u64>,
  pub uid: Option<u32>,
  pub gid: Option<u32>
}

// InodeInfo corresponds to top level dirs
pub struct InodeInfo {
  /// Actuall attr of the virtual file
	pub attr: FileAttr,
  /// Options
  pub options: InodeInfoOptions,
	/// Last update timestamp
	timestamp: SystemTime
}

impl InodeInfo {
	pub fn new(options: InodeInfoOptions) -> io::Result<Self> {
		let attr = InodeInfo::get_metadata(&options)?;
		Ok(Self {
			attr,
      options,
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
			let attr = InodeInfo::get_metadata(&self.options)?;
			self.attr = attr;
			self.timestamp = SystemTime::now();
		}
		Ok(())
	}

	fn get_metadata(options: &InodeInfoOptions) -> io::Result<FileAttr> {
		let src_metadata = fs::metadata(&options.path)?;
		let attr = derive_attr(&src_metadata,	options);
		Ok(attr)
	}
}

// Derive attr from metadata of existing file
pub fn derive_attr(src_metadata: &fs::Metadata, options: &InodeInfoOptions) -> FileAttr {
	let cur_time = SystemTime::now();
	// permission bits (excluding the format bits)
	let mut perm = src_metadata.mode() & !S_IFMT;
	if src_metadata.is_dir() {
		// remove executable bit
		perm &= !(S_IXUSR | S_IXGRP | S_IXOTH);
	}
  let size = options.length.unwrap_or(src_metadata.size().saturating_sub(options.start));

	FileAttr {
		ino: options.ino,
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
		uid: options.uid.unwrap_or(src_metadata.uid()),
		gid: options.gid.unwrap_or(src_metadata.gid()),
		rdev: 0,
		blksize: 512,
		flags: 0 // macOS only
	}
}

