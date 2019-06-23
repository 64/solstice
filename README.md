# ddOS

[![Build Status](https://travis-ci.org/64/ddos.svg?branch=master)](https://travis-ci.org/64/ddos) [![License](https://img.shields.io/badge/license-GPLv3-blue.svg)](https://github.com/64/ddos/blob/master/LICENSE.md)

Rust x86\_64 operating system.

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
