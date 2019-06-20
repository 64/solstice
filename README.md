# ddOS

License: GPLv3

## Install

### Dependencies

Requires nightly rust and QEMU.

```
rustup component add llvm-tools-preview
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
