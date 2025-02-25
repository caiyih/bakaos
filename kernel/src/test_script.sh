#!/bin/busybox
busybox echo "run time-test"
./time-test
busybox echo "run libctest_testcode.sh"
./libctest_testcode.sh
busybox echo "run libc-bench"
./libc-bench
busybox echo "run lua_testcode.sh"
./lua_testcode.sh

cat busybox_cmd.txt | while read line
do
	eval "./busybox $line"
	RTN=$?
    echo ""
	if [[ $RTN -ne 0 && $line != "false" ]] ;then
		echo "testcase busybox $line fail"
		# echo "return: $RTN, cmd: $line" >> $RST
	else
		echo "testcase busybox $line success"
	fi
done
