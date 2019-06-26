#!/bin/bash
# TODO: Use LLVM tools?
x86_64-elf-objdump --no-show-raw-insn -d -Mintel ${1:-target/x86_64-solstice/debug/solstice} | source-highlight -s asm -f esc256 | less
