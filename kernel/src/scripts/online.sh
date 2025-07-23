#!/bin/busybox sh

run_tests() {
    sh interrupts_testcode.sh
    sh copy-file-range_testcode.sh
    sh splice_testcode.sh
}

(cd glibc && run_tests)
(cd musl && run_tests)
