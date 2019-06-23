# Solstice

[![Build Status](https://travis-ci.org/64/solstice.svg?branch=master)](https://travis-ci.org/64/solstice) [![License](https://img.shields.io/badge/license-GPLv3-blue.svg)](https://github.com/64/solstice/blob/master/LICENSE.md)

Rust x86\_64 operating system.

![Img](https://i.imgur.com/1W1r8YX.png)

## Install

### Dependencies

Requires nightly rust and QEMU.

```
rustup component add llvm-tools-preview rust-src
cargo install cargo-xbuild bootimage
```

### Building

```
cargo xbuild
```

### Running

```
cargo xrun
```
