#!/bin/bash

mkdir -p .vscode

cat << EOF > .vscode/settings.json
{
    "rust-analyzer.rustc.source": "discover",
    "rust-analyzer.cargo.target": "riscv64gc-unknown-none-elf",
    "rust-analyzer.check.allTargets": false,
    "rust-analyzer.check.extraArgs": [
        "--target",
        "riscv64gc-unknown-none-elf"
    ],
}
EOF

cat << EOF > .vscode/launch.json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "(debug) Qemu RISC-V64",
            "type": "cppdbg",
            "request": "launch",
            "program": "\${workspaceFolder}/kernel/target/riscv64gc-unknown-none-elf/debug/bakaos",
            "args": [],
            "stopAtEntry": false,
            "cwd": "\${workspaceFolder}/kernel",
            "environment": [],
            "externalConsole": false,
            "MIMode": "gdb",
            "preLaunchTask": "Qemu",
            "miDebuggerPath": "riscv64-elf-gdb",
            "miDebuggerServerAddress": "localhost:1234",
            "hardwareBreakpoints": {
                "require": true,
                "limit": 40
            },
            "setupCommands": [
                {
                    "description": "Enable pretty-printing for gdb",
                    "text": "-enable-pretty-printing",
                    "ignoreFailures": true
                }
            ],
            "useExtendedRemote": true,
        },
    ]
}
EOF

cat << EOF > .vscode/tasks.json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Qemu",
            "type": "shell",
            "command": "make -C kernel debug LOG=TRACE TARGET=riscv64gc-unknown-none-elf",
            "group": {
                "kind": "none",
                "isDefault": true
            },
            "isBackground": true,
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "shared",
                "showReuseMessage": false,
                "clear": true
            },
            "problemMatcher": [
                {
                    "owner": "rust",
                    "fileLocation": ["relative", "\${workspaceFolder}"],
                    "background": {
                        "activeOnStart": true,
                        "beginsPattern": ".",
                        "endsPattern": "."
                    },
                    "pattern": [
                        {
                            "regexp": ".",
                            "file": 1,
                            "line": 2,
                            "column": 3,
                            "severity": 4,
                            "message": 5
                        }
                    ]
                }
            ]
        }
    ]
}
EOF
