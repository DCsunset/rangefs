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

mod rangefs;
mod metadata;

use std::path::PathBuf;
use clap::Parser;
use fuser::{self, MountOption};
use rangefs::RangeFs;

#[derive(Parser)]
#[command(version)]
struct Args {
  /// The source block files to mount
  #[arg(short, long)]
  file: Vec<PathBuf>,

  /// Custom name for each block file
  #[arg(short, long)]
  name: Vec<PathBuf>,

  /// Custom offset from start for each block file (range default to start of file)
  #[arg(short, long)]
  offset: Vec<u64>,

  /// Custom size for each block file (range default to end of file)
  #[arg(short, long)]
  size: Vec<u64>,

  /// Allow other users to access the mounted fs
  #[arg(long)]
  allow_other: bool,

  /// Allow root user to access the mounted fs
  #[arg(long)]
  allow_root: bool,

  /// Timeout for metadata and cache in seconds
  #[arg(short, long, default_value_t = 1)]
  timeout: u64,

  /// Unmount automatically when program exists.
  /// (need --allow-root or --allow-other; auto set one if not specified)
  #[arg(short, long)]
  auto_unmount: bool,

  /// Run in foreground
  #[arg(long)]
  foreground: bool,

  /// Mount point
  mount_point: PathBuf
}

fn main() {
  env_logger::init();
  let args = Args::parse();
  let mut options = vec![
    MountOption::RO,
    MountOption::FSName("rangefs".to_string()),
    MountOption::Subtype("rangefs".to_string()),
  ];
  if args.allow_other {
    options.push(MountOption::AllowOther);
  }
  if args.allow_root {
    options.push(MountOption::AllowRoot);
  }
  if args.auto_unmount {
    options.push(MountOption::AutoUnmount);
  }

  // TODO: support background mount
  fuser::mount2(
    RangeFs::new(
      args.file,
      args.offset,
      args.size,
      args.name,
      args.timeout
    ),
    args.mount_point,
    &options
  ).unwrap();
}
