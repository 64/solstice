#!/bin/sh
# TODO: Use LLVM tools?
x86_64-elf-objdump --no-show-raw-insn -d -Mintel target/x86_64-ddos/debug/ddos | source-highlight -s asm -f esc256 | less
