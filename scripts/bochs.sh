#!/bin/sh
dd if=/dev/zero of=/tmp/ddos.img count=1008 bs=512
dd if=target/x86_64-ddos/debug/bootimage-ddos.bin of=/tmp/ddos.img conv=notrunc
bochs -f scripts/.bochsrc -q
