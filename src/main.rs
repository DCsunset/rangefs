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

extern crate env_logger;

mod rangefs;
mod metadata;

use std::path::PathBuf;
use anyhow::{Result, anyhow};
use clap::Parser;
use fuser::{self, MountOption};
use itertools::Itertools;
use rangefs::RangeFs;
use daemonize::Daemonize;

#[derive(Parser)]
#[command(version)]
struct Args {
  /// source files to map range from
  #[arg(short, long)]
  file: Vec<PathBuf>,

  /// custom name for mounted file
  #[arg(short, long)]
  name: Vec<PathBuf>,

  /// start of the range in file (default to start of file)
  #[arg(short, long)]
  start: Vec<u64>,

  /// length of for range in file (range default to end of file)
  #[arg(short, long)]
  length: Vec<u64>,

  /// uid of the mounted file (default to source uid)
  #[arg(short, long)]
  uid: Vec<u32>,

  /// gid of the mounted file (default to source gid)
  #[arg(short, long)]
  gid: Vec<u32>,

  /// allow other users to access the mounted fs
  #[arg(long)]
  allow_other: bool,

  /// Allow root user to access the mounted fs
  #[arg(long)]
  allow_root: bool,

  /// Unmount automatically when program exists.
  /// (need --allow-root or --allow-other; auto set one if not specified)
  #[arg(short, long)]
  auto_unmount: bool,

  /// Timeout for metadata and cache in seconds
  #[arg(short, long, default_value_t = 1)]
  timeout: u64,

  /// Run in foreground
  #[arg(long)]
  foreground: bool,

  /// Redirect stdout to file (only when in background)
  #[arg(long)]
  stdout: Option<PathBuf>,

  /// Redirect stderr to file (only when in background)
  #[arg(long)]
  stderr: Option<PathBuf>,

  /// comma-separated mount options for compatibility with mount.fuse
  #[arg(short)]
  options: Option<String>,

  /// mount point
  mount_point: PathBuf,

  /// overwrite mount point (original mount_point will be used as fsname)
  overwrite_mount_point: Option<PathBuf>
}

pub fn mount_option_from_str(s: &str) -> MountOption {
  match s {
    "auto_unmount" => MountOption::AutoUnmount,
    "allow_other" => MountOption::AllowOther,
    "allow_root" => MountOption::AllowRoot,
    "default_permissions" => MountOption::DefaultPermissions,
    "dev" => MountOption::Dev,
    "nodev" => MountOption::NoDev,
    "suid" => MountOption::Suid,
    "nosuid" => MountOption::NoSuid,
    "ro" => MountOption::RO,
    "rw" => MountOption::RW,
    "exec" => MountOption::Exec,
    "noexec" => MountOption::NoExec,
    "atime" => MountOption::Atime,
    "noatime" => MountOption::NoAtime,
    "dirsync" => MountOption::DirSync,
    "sync" => MountOption::Sync,
    "async" => MountOption::Async,
    x if x.starts_with("fsname=") => MountOption::FSName(x[7..].into()),
    x if x.starts_with("subtype=") => MountOption::Subtype(x[8..].into()),
    x => MountOption::CUSTOM(x.into()),
  }
}

fn main() -> Result<()> {
  let env = env_logger::Env::default()
    .filter_or("RANGEFS_LOG", "warn")
    .write_style("RANGEFS_LOG_STYLE");
  env_logger::init_from_env(env);

  let mut args = Args::parse();
  let fs_name = match &args.overwrite_mount_point {
    Some(_) => Some(&args.mount_point),
    None => None
  };
  // use fs_name as the file if no file is specified
  if fs_name.is_some() && args.file.is_empty() {
    args.file.push(fs_name.unwrap().into());
  }
  let mut options = vec![
    MountOption::RO,
    MountOption::FSName(fs_name.map(|v| v.to_string_lossy().into()).unwrap_or("rangefs".into())),
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
  if let Some(opt) = args.options {
    for o in opt.split(',').map(mount_option_from_str) {
      match o {
        MountOption::RW => (),
        MountOption::CUSTOM(x) => {
          let parts: Vec<_> = x.split("=").collect();
          match parts[0] {
            "file" => args.file = parts[1].split(" ").map_into().collect(),
            "name" => args.name = parts[1].split(" ").map_into().collect(),
            "start" => args.start = parts[1].split(" ").map(str::parse).collect::<Result<_, _>>()?,
            "length" => args.length = parts[1].split(" ").map(str::parse).collect::<Result<_, _>>()?,
            "uid" => args.uid = parts[1].split(" ").map(str::parse).collect::<Result<_, _>>()?,
            "gid" => args.gid = parts[1].split(" ").map(str::parse).collect::<Result<_, _>>()?,
            "timeout" => args.timeout = parts[1].parse()?,
            "stdout" => args.stdout = Some(parts[1].into()),
            "stderr" => args.stderr = Some(parts[1].into()),
            _ => options.push(MountOption::CUSTOM(x))
          };
        },
        x => {
          options.push(x);
        },
      };
    }
  }

  if args.file.is_empty() {
    return Err(anyhow!("No source file specified"));
  }

  let mount_point = args.overwrite_mount_point.unwrap_or(args.mount_point);
  if !mount_point.as_path().is_dir() {
    return Err(anyhow!("Mount point doesn't exist or isn't a directory"));
  }

  let mount_fs = || {
    fuser::mount2(
      RangeFs::new(
        &args.file,
        &args.name,
        &args.start,
        &args.length,
        &args.uid,
        &args.gid,
        args.timeout
      ),
      &mount_point,
      &options
    )
  };

  if args.foreground {
    mount_fs()?;
  } else {
    let mut daemon = Daemonize::new().working_directory(".");
    if let Some(stdout) = args.stdout {
      daemon = daemon.stdout(std::fs::File::create(stdout)?);
    }
    if let Some(stderr) = args.stderr {
      daemon = daemon.stderr(std::fs::File::create(stderr)?);
    }

    match daemon.start() {
      Ok(_) => mount_fs()?,
      Err(e) => return Err(anyhow!("error creating daemon: {}", e))
    };
  }

  Ok(())
}
