# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

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
[Unreleased]: https://github.com/kaspar030/git-cache-rs/compare/0.2.3...HEAD
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
