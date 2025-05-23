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

mod rangefs;
mod metadata;

use std::{fs, path::PathBuf};
use anyhow::anyhow;
use clap::Parser;
use fuser::{self, MountOption};
use rangefs::RangeFs;
use metadata::InodeConfig;
use ipc_channel::ipc;
use nix::unistd::{dup2_stderr, dup2_stdout, fork, ForkResult};

#[derive(Parser)]
#[command(version)]
struct Args {
  /// Config string for each mapped file with colon-separated options
  /// Supported options:
  /// - offset=<offset> (default: 0)
  /// - size=<size> (default: source_file_size - offset)
  /// - name=<mapped_filename> (default: source_filename)
  /// - uid=<uid> (default: source_uid)
  /// - gid=<gid> (default: source_gid)
  /// - preload (default: false)
  #[arg(short, long, verbatim_doc_comment)]
  config: Vec<String>,

  /// Timeout for metadata and cache in seconds
  #[arg(short, long, default_value_t = 1)]
  timeout: u64,

  /// Redirect stdout to file (only when in background)
  #[arg(long)]
  stdout: Option<PathBuf>,

  /// Redirect stderr to file (only when in background)
  #[arg(long)]
  stderr: Option<PathBuf>,

  /// Run in foreground
  #[arg(long)]
  foreground: bool,

  /// comma-separated mount options for compatibility with mount.fuse and fstab
  #[arg(short)]
  options: Option<String>,

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

  /// Overwrite source file (useful for customizing fsname)
  #[arg(short, long)]
  file: Option<PathBuf>,

  /// source file to map ranges from
  source: PathBuf,

  /// mount point
  mount_point: PathBuf
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

pub fn parse_config(config_str: impl AsRef<str>) -> anyhow::Result<InodeConfig> {
  let assert_opt = |cond: bool, opt_str| -> anyhow::Result<()> {
    if !cond {
      Err(anyhow!("invalid option: {}", opt_str))
    } else {
      Ok(())
    }
  };

  let mut config = InodeConfig::default();
  if config_str.as_ref().is_empty() {
    // use default config
    return Ok(config);
  }
  for opt_str in config_str.as_ref().split(":") {
    let parts: Vec<_> = opt_str.split("=").collect();
    assert_opt(parts.len() >= 1 && parts.len() <= 2, opt_str)?;
    match parts[0] {
      "name" => config.name = Some(parts[1].into()),
      "offset" => config.offset = Some(parts[1].parse()?),
      "size" => config.size = Some(parts[1].parse()?),
      "uid" => config.uid = Some(parts[1].parse()?),
      "gid" => config.gid = Some(parts[1].parse()?),
      "preload" => config.preload = true,
      _ => assert_opt(false, opt_str)?
    };
  }
  Ok(config)
}

fn main() -> anyhow::Result<()> {
  let env = env_logger::Env::default()
    .filter_or("RANGEFS_LOG", "warn")
    .write_style("RANGEFS_LOG_STYLE");
  env_logger::init_from_env(env);

  let args = Args::parse();
  let mut options = vec![
    MountOption::RO,
    MountOption::FSName(args.source.to_string_lossy().into()),
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

  let mut file = args.file;
  let mut timeout = args.timeout;
  let mut configs = args.config.iter().map(parse_config).collect::<Result<Vec<_>, _>>()?;
  let mut stdout = args.stdout;
  let mut stderr = args.stderr;

  if let Some(opt) = args.options {
    for o in opt.split(',').map(mount_option_from_str) {
      match o {
        MountOption::RW => (),
        MountOption::CUSTOM(x) => {
          match x {
            x if x.starts_with("config::") => {
              for c in x.split("::").skip(1).map(parse_config) {
                configs.push(c?);
              }
            },
            x if x.starts_with("file::") => {
              file = Some(x.split("::").skip(1).next().ok_or(anyhow!("invalid option: {}", x))?.into());
            },
            x if x.starts_with("timeout::") => {
              timeout = x.split("::").skip(1).next().ok_or(anyhow!("invalid option: {}", x))?.parse()?;
            },
            x if x.starts_with("stdout::") => {
              stdout = Some(x.split("::").skip(1).next().ok_or(anyhow!("invalid option: {}", x))?.into());
            },
            x if x.starts_with("stderr::") => {
              stderr = Some(x.split("::").skip(1).next().ok_or(anyhow!("invalid option: {}", x))?.into());
            },
            _ => options.push(MountOption::CUSTOM(x))
          };
        },
        x => {
          options.push(x);
        },
      };
    }
  }

  if configs.is_empty() {
    return Err(anyhow!("no mapping config specified"));
  }

  if !args.mount_point.as_path().is_dir() {
    return Err(anyhow!("mount point doesn't exist or isn't a directory"));
  }

  let mount_fs = |tx| {
    fuser::mount2(
      RangeFs::new(
        file.unwrap_or(args.source),
        configs,
        timeout,
        tx,
      ),
      &args.mount_point,
      &options
    )
  };

  if args.foreground {
    mount_fs(None)?;
  } else {
    let (tx, rx) = ipc::channel::<Option<String>>()?;

    match unsafe { fork()? } {
      ForkResult::Parent { .. } => {
        // Wait until mounted
        if let Some(msg) = rx.recv()? {
          // Error msg
          return Err(anyhow!(msg));
        }
      },
      ForkResult::Child => {
        let exec = || -> anyhow::Result<()> {
          if let Some(stdout) = stdout {
            dup2_stdout(fs::File::create(stdout)?)?;
          }
          if let Some(stderr) = stderr {
            dup2_stderr(fs::File::create(stderr)?)?;
          }
          mount_fs(Some(tx.clone()))?;

          Ok(())
        };

        if let Err(e) = exec() {
          // Must send msg to unblock parent
          tx.send(Some(e.to_string()))?;
        }
      }
    }
  }

  Ok(())
}
