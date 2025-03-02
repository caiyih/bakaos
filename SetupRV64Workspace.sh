#!/bin/bash

mkdir -p .vscode

if [ ! -f .vscode/settings.json ]; then
    echo "{}" > .vscode/settings.json
fi

jq '. + {
    "rust-analyzer.rustc.source": "discover",
    "rust-analyzer.cargo.target": "riscv64gc-unknown-none-elf",
    "rust-analyzer.check.allTargets": false,
    "rust-analyzer.check.extraArgs": [
        "--target",
        "riscv64gc-unknown-none-elf"
    ],
}' .vscode/settings.json > .vscode/settings.json.tmp

mv .vscode/settings.json.tmp .vscode/settings.json
