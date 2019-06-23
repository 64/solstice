#!/bin/sh
dd if=/dev/zero of=/tmp/solstice.img count=1008 bs=512
dd if=target/x86_64-solstice/debug/bootimage-solstice.bin of=/tmp/solstice.img conv=notrunc
bochs -f scripts/.bochsrc -q
