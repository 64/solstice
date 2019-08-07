#!/bin/sh
set -me
bootimage build
qemu-system-x86_64 -drive format=raw,file=target/x86_64-solstice/debug/bootimage-solstice.bin -no-reboot -S -s &
sleep 0.5
gdb target/x86_64-solstice/debug/solstice -ex "target remote :1234"
pkill qemu
