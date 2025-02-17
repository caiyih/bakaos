#!/bin/busybox
busybox echo "run time-test"
./time-test
busybox echo "run libctest_testcode.sh"
./libctest_testcode.sh
busybox echo "run libc-bench"
./libc-bench
busybox echo "run lua_testcode.sh"
./lua_testcode.sh

test_busybox_command() {
    local line="$1"
    eval "./busybox $line"
    local RTN=$?
    if [[ $RTN -ne 0 && $line != "false" ]]; then
        echo "\ntestcase busybox $line fail"
        # echo "return: $RTN, cmd: $line" >> $RST
    else
        echo "\ntestcase busybox $line success"
    fi
}

test_busybox_command "echo \"#### independent command test\""
test_busybox_command "ash -c exit"
test_busybox_command "sh -c exit"
test_busybox_command "basename /aaa/bbb"
test_busybox_command "cal"
test_busybox_command "clear"
test_busybox_command "date"
test_busybox_command "df"
test_busybox_command "dirname /aaa/bbb"
test_busybox_command "dmesg"
test_busybox_command "du"
test_busybox_command "expr 1 + 1"
test_busybox_command "false"
test_busybox_command "true"
test_busybox_command "which ls"
test_busybox_command "uname"
test_busybox_command "uptime"
test_busybox_command "printf \"abc\n\""
test_busybox_command "ps"
test_busybox_command "pwd"
test_busybox_command "free"
test_busybox_command "hwclock"
test_busybox_command "kill 10"
test_busybox_command "ls"
test_busybox_command "sleep 1"

test_busybox_command "echo \"#### file opration test\""
test_busybox_command "touch test.txt"
test_busybox_command "echo \"hello world\" > test.txt"
test_busybox_command "cat test.txt"
test_busybox_command "cut -c 3 test.txt"
test_busybox_command "od test.txt"
test_busybox_command "head test.txt"
test_busybox_command "tail test.txt"
test_busybox_command "hexdump -C test.txt"
test_busybox_command "md5sum test.txt"
test_busybox_command "echo \"ccccccc\" >> test.txt"
test_busybox_command "echo \"bbbbbbb\" >> test.txt"
test_busybox_command "echo \"aaaaaaa\" >> test.txt"
test_busybox_command "echo \"2222222\" >> test.txt"
test_busybox_command "echo \"1111111\" >> test.txt"
test_busybox_command "echo \"bbbbbbb\" >> test.txt"
test_busybox_command "sort test.txt | ./busybox uniq"
test_busybox_command "stat test.txt"
test_busybox_command "strings test.txt"
test_busybox_command "wc test.txt"
test_busybox_command "[ -f test.txt ]"
test_busybox_command "more test.txt"
test_busybox_command "rm test.txt"
test_busybox_command "mkdir test_dir"
test_busybox_command "mv test_dir test"
test_busybox_command "rmdir test"
test_busybox_command "grep hello busybox_cmd.txt"
test_busybox_command "cp busybox_cmd.txt busybox_cmd.bak"
test_busybox_command "rm busybox_cmd.bak"
test_busybox_command "find -name \"busybox_cmd.txt\""
