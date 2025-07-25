#!/bin/bash

mkdir -p .vscode

ARCH=loongarch64 # Reserved for future use, pass to makefile to determine qemu executable
TARGET=loongarch64-unknown-none
GDB=loongarch64-linux-gnu-gdb

cat << EOF > .vscode/settings.json
{
    "rust-analyzer.rustc.source": "discover",
    "rust-analyzer.cargo.target": "${TARGET}",
    "rust-analyzer.check.allTargets": false,
    "rust-analyzer.check.extraArgs": [
        "--target",
        "${TARGET}"
    ],
    "rust-analyzer.files.exclude": [
        "source-generation"
    ],
}
EOF
