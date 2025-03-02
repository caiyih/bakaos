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
}
EOF

cat << EOF > .vscode/launch.json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "(debug) Qemu ${ARCH}",
            "type": "cppdbg",
            "request": "launch",
            "program": "\${workspaceFolder}/kernel/target/${TARGET}/debug/bakaos",
            "args": [],
            "stopAtEntry": false,
            "cwd": "\${workspaceFolder}/kernel",
            "environment": [],
            "externalConsole": false,
            "MIMode": "gdb",
            "preLaunchTask": "Qemu-debug",
            "miDebuggerPath": "${GDB}",
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
        {
            "name": "(release) Qemu ${ARCH}",
            "type": "cppdbg",
            "request": "launch",
            "program": "\${workspaceFolder}/kernel/target/${TARGET}/release-with-debug/bakaos",
            "args": [],
            "stopAtEntry": false,
            "cwd": "\${workspaceFolder}/kernel",
            "environment": [],
            "externalConsole": false,
            "MIMode": "gdb",
            "preLaunchTask": "Qemu-release",
            "miDebuggerPath": "${GDB}",
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
            "label": "Qemu-debug",
            "type": "shell",
            "command": "make -C kernel debug LOG=TRACE TARGET=${TARGET}",
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
        },
        {
            "label": "Qemu-release",
            "type": "shell",
            "command": "make -C kernel debug MODE=release-with-debug LOG=TRACE TARGET=${TARGET}",
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
        },
    ]
}
EOF
