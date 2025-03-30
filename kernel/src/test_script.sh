#!/bin/busybox
(
	cd basic
	echo "#### OS COMP TEST GROUP START basic-musl ####"
	./run-all.sh
    ./sleep
    ./yield
	echo "#### OS COMP TEST GROUP END basic-musl ####"
)
./lua_testcode.sh

echo "#### OS COMP TEST GROUP START busybox-glibc ####"
cat ./busybox_cmd.txt | while read line
do
        eval "./busybox $line"
        RTN=$?
        if [[ $RTN -ne 0 && $line != "false" ]] ;then
                echo ""
                echo "testcase busybox $line fail"
        else
                echo "testcase busybox $line success"
        fi
done

echo "#### OS COMP TEST GROUP END busybox-glibc ####"

./libctest_testcode.sh
cyclictest_testcode.sh
