# rangefs

[![crates.io](https://badgen.net/crates/v/rangefs)](https://crates.io/crates/rangefs)

A fuse-based filesystem to access block devices as normal files.


## Installation

Pre-built binaries are available at the GitHub release page.

You can also use cargo to install it:

```sh
cargo install rangefs
```

## Usage

To mount files with range to a mount point:

```sh
# mount
rangefs --file file1 --offset 0 --size 8 <mount_point>
# multiple files
rangefs -f file1 -o 1 -f file2 -o 2 <mount_point>
# rename mounted files
rangefs -f file1 -o 1 -n f1 -f file1 -o 2 -n f2 <mount_point>

# unmount
fusermount -u <mount_point>
```

The mount point will be a read-only filesystem containing files that correponding to the specified ranges in source files.

Note that the program will run in the background by default.
Add flag `--fg` to run it in the foreground.

If the program exits without using `fusermount`,
`fusermoutn` still needs to be used even after the program exits.
You can also use `-a` option to auto unmount the fs upon program exit.

Note that =rangefs= also supports block special file.
However, you need to speicify the size of the range.
Otherwise, the default size will be 0 (same as that in the block file metadata).

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
