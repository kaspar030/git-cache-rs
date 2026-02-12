# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

### 🐛 Bug Fixes

- Use no-cone for sparse checkout

### ⚙️ Miscellaneous Tasks

- Bump MSRV to 1.82
- *(ci)* Update Debian/Ubuntu container builds

### 💼 Other

- *(deps)* Bump anyhow from 1.0.100 to 1.0.101
- *(deps)* Bump clap from 4.5.49 to 4.5.58
- *(deps)* Bump gix-config from 0.46.0 to 0.52.0
- *(deps)* Bump url from 2.5.7 to 2.5.8

## [0.2.6] - 2025-10-20

### 🚀 Features

- Add `prefetch` command

### 🐛 Bug Fixes

- Actually implement recursive pre-fetching

### 💼 Other

- *(deps)* Bump anyhow from 1.0.97 to 1.0.98
- *(deps)* Bump shellexpand from 3.1.0 to 3.1.1
- *(deps)* Bump clap from 4.5.36 to 4.5.37
- *(deps)* Bump gix-config from 0.44.0 to 0.45.0
- *(deps)* Bump gix-config from 0.45.0 to 0.45.1
- *(deps)* Bump clap from 4.5.38 to 4.5.39
- *(deps)* Bump camino from 1.1.9 to 1.1.10
- *(deps)* Bump clap from 4.5.39 to 4.5.40
- *(deps)* Bump clap from 4.5.40 to 4.5.41
- *(deps)* Bump gix-config from 0.45.1 to 0.46.0
- *(deps)* Bump clap from 4.5.41 to 4.5.42
- *(deps)* Bump clap from 4.5.42 to 4.5.43
- *(deps)* Bump camino from 1.1.10 to 1.1.11
- *(deps)* Bump clap from 4.5.43 to 4.5.44
- *(deps)* Bump clap from 4.5.44 to 4.5.45
- *(deps)* Bump anyhow from 1.0.98 to 1.0.99
- *(deps)* Bump rayon from 1.10.0 to 1.11.0
- *(deps)* Bump url from 2.5.4 to 2.5.6
- *(deps)* Bump url from 2.5.6 to 2.5.7
- *(deps)* Bump camino from 1.1.11 to 1.1.12
- *(deps)* Bump clap from 4.5.45 to 4.5.47
- *(deps)* Bump clap from 4.5.47 to 4.5.48
- *(deps)* Bump anyhow from 1.0.99 to 1.0.100
- *(deps)* Bump clap from 4.5.48 to 4.5.49

### 🚜 Refactor

- Use enum instead of string matching for prefetch channel

### ⚙️ Miscellaneous Tasks

- Bump workflow versions
- Bump deps. use rust 1.82 for buster containers

## [0.2.5] - 2025-04-13

### Changed

- some dependency upgrades

## [0.2.4] - 2025-01-27

- implement recursive submodule cloning from cache

## [0.2.3] - 2024-07-01

## [0.2.2] - 2024-07-01

#### Changed

- don't cache clones from local repositories

## [0.2.1] - 2024-06-28

#### Fixed

- don't chdir into cache repo path on initial clone. This fixes clones from
  relative paths (`git cache clone ./repo target_path`)

## [0.2.0] - 2024-06-28

#### Changed

- split into library and binary

## [0.1.5] - 2024-02-06

#### Fixed

- don't forget checking out commit on first mirror
- check if folder exists in `is_initialized()`

#### Changed

- strip & LTO by default

## [0.1.4] - 2024-02-02

#### Fixed

- always create cache folder. This prevents a panic creating a repository
  lockfile.
- added context to some errors

## [0.1.3] - 2024-01-30

#### Changed

- don't error on `git-cache init`

## [0.1.2] - 2024-01-30

#### Fixed

- don't set "core.compression=true" on mirror repo. Fixes `git < 2.39`.

## [0.1.1] - 2024-01-30

#### Added

- implemented basic cache repository locking

## [0.1.0] - 29.01.2024

<!-- next-url -->
[Unreleased]: https://github.com/kaspar030/git-cache-rs/compare/0.2.6...HEAD
[0.2.6]: https://github.com/kaspar030/git-cache-rs/compare/0.2.5...0.2.6

[0.2.5]: https://github.com/kaspar030/git-cache-rs/compare/0.2.4...0.2.5
[0.2.4]: https://github.com/kaspar030/git-cache-rs/compare/0.2.3...0.2.4
[0.2.3]: https://github.com/kaspar030/git-cache-rs/compare/0.2.2...0.2.3
[0.2.2]: https://github.com/kaspar030/git-cache-rs/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/kaspar030/git-cache-rs/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/kaspar030/git-cache-rs/compare/0.1.5...0.2.0
[0.1.5]: https://github.com/kaspar030/git-cache-rs/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/kaspar030/git-cache-rs/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/kaspar030/git-cache-rs/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/kaspar030/git-cache-rs/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/kaspar030/git-cache-rs/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/kaspar030/git-cache-rs/releases/tag/0.1.0
