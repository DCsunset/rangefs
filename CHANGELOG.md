# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.

## [0.3.1](https://github.com/DCsunset/rangefs/compare/v0.3.0...v0.3.1) (2023-12-26)


### Bug Fixes

* fix some warnings ([8aa47ba](https://github.com/DCsunset/rangefs/commit/8aa47ba40ea2f9ab09617cbfca014ee92bc4792d))

## [0.3.0](https://github.com/DCsunset/rangefs/compare/v0.2.0...v0.3.0) (2023-12-26)


### ⚠ BREAKING CHANGES

* rename options to prevent conflict with mount options

### Features

* add placeholder when source file doesn't exist ([92eb18b](https://github.com/DCsunset/rangefs/commit/92eb18b8f62d3bac0b1047bf06c39eaac546a581))


### Bug Fixes

* rename options to prevent conflict with mount options ([d84e9ed](https://github.com/DCsunset/rangefs/commit/d84e9edda62f6ba9460b461be3fb6d3cf592e274))

## [0.2.0](https://github.com/DCsunset/rangefs/compare/v0.1.0...v0.2.0) (2023-12-26)


### ⚠ BREAKING CHANGES

* support mount.fuse through -o option

### Features

* support mount.fuse through -o option ([55fe171](https://github.com/DCsunset/rangefs/commit/55fe1718039e8ea3d16ae98999c7f8304a68f2ca))


### Bug Fixes

* use root id constant from fuser ([9031703](https://github.com/DCsunset/rangefs/commit/9031703a7537e3961c5f4793264422fa85eb74f4))

## 0.1.0 (2023-12-25)


### Features

* calculate fs size for statfs ([d960bac](https://github.com/DCsunset/rangefs/commit/d960bac9439feb6867fc58c2967d0942ca3e1a79))
* make range default to end of file if size not specified ([5f2ef7f](https://github.com/DCsunset/rangefs/commit/5f2ef7f579108d2314efda062b9087dbffc50271))
* support overwriting uid and gid ([de5e75a](https://github.com/DCsunset/rangefs/commit/de5e75aa147be0a0833f1c575c621da1d5013031))
* support running rangefs in background ([61f7b3b](https://github.com/DCsunset/rangefs/commit/61f7b3b1d76877480e2b658c8b7850cbefe7573b))


### Bug Fixes

* use custom env for logging and fix working directory ([37eae11](https://github.com/DCsunset/rangefs/commit/37eae1107b25373cececd156f9fdc9ffec48acff))
