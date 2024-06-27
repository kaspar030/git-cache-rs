# Introduction

**git-cache-rs** is a git helper that allows caching clones in a central folder
so consecutive clones become faster.

It works by first cloning into a cache folder (`~/.gitcache` by default), then
cloning locally out from there. The next time the same repository is cloned, it
will be cloned from the cache.

## Installation

    cargo install git-cache

## How to use

Just use `git cache clone <clone options>` instead of `git clone <clone
options>`. Add `-U` if you'd like the cached version to update from the
original repository before cloning (not needed for the first clone).

## License

git-cache-rs is licensed under the terms of the Apache License (Version 2.0).
