#!/bin/busybox
(
	cd basic
	echo "#### OS COMP TEST GROUP START basic-musl ####"
	./run-all.sh
	echo "#### OS COMP TEST GROUP END basic-musl ####"
)
./lua_testcode.sh
./libctest_testcode.sh
./busybox_testcode.sh
