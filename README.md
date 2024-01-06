# rangefs

[![crates.io](https://badgen.net/crates/v/rangefs)](https://crates.io/crates/rangefs)

A FUSE-based read-only filesystem to map ranges in file to individual files.

## Installation

Pre-built binaries are available at the GitHub release page.

You can also use cargo to install it:

```sh
cargo install rangefs
```

If you are using Nix, you can also install it from NUR package `nur.repos.dcsunset.rangefs`.

## Usage

To mount files with range to a mount point:

```sh
# mount a range as a new file
rangefs --config offset=16:size=16 <file> <mount_point>
# multiple ranges with different names
rangefs -c offset=4:name=range1 -c offset=8:size=8:name=range2 <file> <mount_point>

# unmount
fusermount -u <mount_point>

# To adjust log level and run at foreground
RANGEFS_LOG=debug rangefs -c offset=1:size=1 --foreground <file> <mount_point>
```

The mount point will be a read-only filesystem containing files that correponding to the specified ranges in the source file.
Repeat the `--config` option to mount multiple ranges.

Note that the program will run in the background by default.
Use flag `--foreground` to run it in the foreground.

If the program exits without using `fusermount`,
`fusermoutn` still needs to be used even after the program exits.
You can also use `-a` option to auto unmount the fs upon program exit.

Note that rangefs also supports block special file.
However, you need to speicify the length of the range.
Otherwise, the default length will be 0 (same as the size in the block file metadata).

Rangefs also supports mounting through `mount.fuse` or `/etc/fstab`.
To specify configs, start with `config::` and separate configs by double colons.
For timeout, stdout and stderr, speicify `<option>::<value>` to set it.
`::` is used instead of `=` to distinguish custom options from existing mount options.
An example fstab config:
```
/source_file /mount_point fuse./path/to/rangefs nofail,allow_other,config::name=r1:offset=1::name=r2:offset=2:size=2 0 0
```

See available options using `rangefs --help`.

## License

AGPL-3.0. Copyright notice:

```
rangefs
Copyright (C) 2023  DCsunset

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
