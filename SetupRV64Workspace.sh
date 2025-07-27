#!/bin/bash

mkdir -p .vscode

ARCH=riscv64 # Reserved for future use, pass to makefile to determine qemu executable
TARGET=riscv64gc-unknown-none-elf
GDB=riscv64-elf-gdb

cat << EOF > .vscode/settings.json
{
    "rust-analyzer.rustc.source": "discover",
    "rust-analyzer.cargo.target": "${TARGET}",
    "rust-analyzer.check.allTargets": false,
    "rust-analyzer.check.noDefaultFeatures": true,
    "rust-analyzer.cargo.noDefaultFeatures": true,
    "rust-analyzer.cargo.features": ["virt"],
    "rust-analyzer.check.features": ["virt"],
    "rust-analyzer.check.extraArgs": [
        "--target",
        "${TARGET}"
    ],
    "rust-analyzer.files.exclude": [
        "source-generation"
    ],
}
EOF
