#!/usr/bin/sh

loongarch64-linux-musl-gcc -static -nostdlib hello-la.S -o hello-la
riscv64-linux-musl-gcc -nostdlib -static -no-pie hello-rv.S -o hello-rv
