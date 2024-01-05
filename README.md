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
# mount
rangefs --file file1 --start 0 --length 8 <mount_point>
# multiple files
rangefs -f file1 -s 1 -f file2 -s 2 <mount_point>
# rename mounted files
rangefs -f file1 -s 1 -n f1 -f file1 -s 2 -n f2 <mount_point>

# unmount
fusermount -u <mount_point>

# To adjust log level
RANGEFS_LOG=info rangefs -f file1 -s 1 -n f1 --foreground <mount_point>
```

The mount point will be a read-only filesystem containing files that correponding to the specified ranges in source files.
To mount multiple files, the following options can be repeated to configure different sources:
`-f`, `-n`, `-s`, `-l`, `-u`, `-g`.

Note that the program will run in the background by default.
Use flag `--foreground` to run it in the foreground.

If the program exits without using `fusermount`,
`fusermoutn` still needs to be used even after the program exits.
You can also use `-a` option to auto unmount the fs upon program exit.

Note that rangefs also supports block special file.
However, you need to speicify the length of the range.
Otherwise, the default length will be 0 (same as the size in the block file metadata).

Rangefs also supports mounting through `mount.fuse` or `/etc/fstab`.
In this case, the first positional argument is used as fsname and the second argument is the actual mount point.
For options that can accept multiple values, use spaces to separate them
(repeating them won't work as mount will merge duplicate options)
An example fstab config:
```
rangefs /mount_point fuse./path/to/rangefs nofail,allow_other,file="file1 file2",start="1 2",uid=1000,length=16 0 0
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
