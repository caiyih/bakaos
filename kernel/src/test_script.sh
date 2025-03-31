#!/bin/busybox

run_test() {
    local test_dir="$1"
    local test_function="$2"
    local test_name="$3"
    local is_print_marker="${4:-false}"
    local arch="$5"

    if [ -d "$test_dir" ]; then
        (
            cd "$test_dir" || exit

            if [[ "$is_print_marker" == "true" ]]; then
                echo "#### OS COMP TEST GROUP START ${test_name}-${test_dir} ####"
            fi

            "$test_function" "$test_dir" "$arch"

            if [[ "$is_print_marker" == "true" ]]; then
                echo "#### OS COMP TEST GROUP END ${test_name}-${test_dir} ####"
            fi
        )
    else
        echo "Directory $test_dir does not exist!"
    fi
}


basic_test() {
    cd ./basic
    ./run-all.sh
    ./sleep
    ./yield
}

busybox_test() {
    while read -r line; do
        eval "./busybox $line"
        RTN=$?
        if [[ $RTN -ne 0 && $line != "false" ]]; then
            echo ""
            echo "testcase busybox $line fail"
        else
            echo "testcase busybox $line success"
        fi
    done < ./busybox_cmd.txt
}

libc_test() {
    local libc="$1"
    local arch="$2"

    if [[ "$arch" == "rv" ]]; then
        libc_test_rv_static "$libc"
    fi

    if [[ "$arch" == "la" ]]; then
        libc_test_la_static "$libc"
    fi

    ./run-dynamic.sh
}

libc_test_rv_static() {
    local libc="$1"

    if [[ "$libc" == "musl" ]]; then
        ./run-static.sh
    fi

    if [[ "$libc" == "glibc" ]]; then
        libc_test_static_run_case argv
        libc_test_static_run_case basename
        # libc_test_static_run_case clocale_mbfuncs
        libc_test_static_run_case clock_gettime
        libc_test_static_run_case dirname
        libc_test_static_run_case env
        libc_test_static_run_case fdopen
        # libc_test_static_run_case fnmatch
        # libc_test_static_run_case fscanf
        # libc_test_static_run_case fwscanf
        libc_test_static_run_case iconv_open
        libc_test_static_run_case inet_pton
        # libc_test_static_run_case mbc
        libc_test_static_run_case memstream
        # libc_test_static_run_case pthread_cancel_points
        libc_test_static_run_case pthread_cancel
        libc_test_static_run_case pthread_cond
        libc_test_static_run_case pthread_tsd
        libc_test_static_run_case qsort
        libc_test_static_run_case random
        libc_test_static_run_case search_hsearch
        libc_test_static_run_case search_insque
        libc_test_static_run_case search_lsearch
        libc_test_static_run_case search_tsearch
        # libc_test_static_run_case setjmp
        # libc_test_static_run_case snprintf
        # libc_test_static_run_case socket
        # libc_test_static_run_case sscanf
        libc_test_static_run_case sscanf_long
        # libc_test_static_run_case stat
        # libc_test_static_run_case strftime
        libc_test_static_run_case string
        libc_test_static_run_case string_memcpy
        libc_test_static_run_case string_memmem
        libc_test_static_run_case string_memset
        libc_test_static_run_case string_strchr
        libc_test_static_run_case string_strcspn
        libc_test_static_run_case string_strstr
        libc_test_static_run_case strptime
        libc_test_static_run_case strtod
        libc_test_static_run_case strtod_simple
        libc_test_static_run_case strtof
        # libc_test_static_run_case strtol
        libc_test_static_run_case strtold
        # libc_test_static_run_case swprintf
        libc_test_static_run_case tgmath
        libc_test_static_run_case time
        libc_test_static_run_case tls_align
        libc_test_static_run_case udiv
        # libc_test_static_run_case ungetc
        # libc_test_static_run_case utime
        # libc_test_static_run_case wcsstr
        # libc_test_static_run_case wcstol
        # libc_test_static_run_case daemon_failure
        # libc_test_static_run_case dn_expand_empty
        # libc_test_static_run_case dn_expand_ptr_0
        libc_test_static_run_case fflush_exit
        libc_test_static_run_case fgets_eof
        # libc_test_static_run_case fgetwc_buffering
        libc_test_static_run_case fpclassify_invalid_ld80
        libc_test_static_run_case ftello_unflushed_append
        libc_test_static_run_case getpwnam_r_crash
        libc_test_static_run_case getpwnam_r_errno
        libc_test_static_run_case iconv_roundtrips
        libc_test_static_run_case inet_ntop_v4mapped
        libc_test_static_run_case inet_pton_empty_last_field
        libc_test_static_run_case iswspace_null
        libc_test_static_run_case lrand48_signextend
        libc_test_static_run_case lseek_large
        libc_test_static_run_case malloc_0
        libc_test_static_run_case mbsrtowcs_overflow
        libc_test_static_run_case memmem_oob_read
        libc_test_static_run_case memmem_oob
        libc_test_static_run_case mkdtemp_failure
        libc_test_static_run_case mkstemp_failure
        libc_test_static_run_case printf_1e9_oob
        libc_test_static_run_case printf_fmt_g_round
        libc_test_static_run_case printf_fmt_g_zeros
        libc_test_static_run_case printf_fmt_n
        libc_test_static_run_case pthread_robust_detach
        libc_test_static_run_case pthread_cancel_sem_wait
        libc_test_static_run_case pthread_cond_smasher
        # libc_test_static_run_case pthread_condattr_setclock
        libc_test_static_run_case pthread_exit_cancel
        libc_test_static_run_case pthread_once_deadlock
        libc_test_static_run_case pthread_rwlock_ebusy
        libc_test_static_run_case putenv_doublefree
        libc_test_static_run_case regex_backref_0
        libc_test_static_run_case regex_bracket_icase
        # libc_test_static_run_case regex_ere_backref
        # libc_test_static_run_case regex_escaped_high_byte
        libc_test_static_run_case regex_negated_range
        libc_test_static_run_case regexec_nosub
        libc_test_static_run_case rewind_clear_error
        libc_test_static_run_case rlimit_open_files
        libc_test_static_run_case scanf_bytes_consumed
        libc_test_static_run_case scanf_match_literal_eof
        libc_test_static_run_case scanf_nullbyte_char
        # libc_test_static_run_case setvbuf_unget
        libc_test_static_run_case sigprocmask_internal
        libc_test_static_run_case sscanf_eof
        # libc_test_static_run_case statvfs
        libc_test_static_run_case strverscmp
        libc_test_static_run_case syscall_sign_extend
        libc_test_static_run_case uselocale_0
        libc_test_static_run_case wcsncpy_read_overflow
        libc_test_static_run_case wcsstr_false_negative
    fi
}


libc_test_la_static() {
    local libc="$1"

    if [[ "$libc" == "glibc" ]]; then
        ./run-static.sh
    fi

    if [[ "$libc" == "musl" ]]; then
        libc_test_static_run_case argv
        libc_test_static_run_case basename
        libc_test_static_run_case clocale_mbfuncs
        libc_test_static_run_case clock_gettime
        libc_test_static_run_case dirname
        libc_test_static_run_case env
        libc_test_static_run_case fdopen
        libc_test_static_run_case fnmatch
        libc_test_static_run_case fscanf
        libc_test_static_run_case fwscanf
        libc_test_static_run_case iconv_open
        libc_test_static_run_case inet_pton
        libc_test_static_run_case mbc
        libc_test_static_run_case memstream
        # libc_test_static_run_case pthread_cancel_points
        # libc_test_static_run_case pthread_cancel
        # libc_test_static_run_case pthread_cond
        libc_test_static_run_case pthread_tsd
        libc_test_static_run_case qsort
        libc_test_static_run_case random
        libc_test_static_run_case search_hsearch
        libc_test_static_run_case search_insque
        libc_test_static_run_case search_lsearch
        libc_test_static_run_case search_tsearch
        libc_test_static_run_case setjmp
        libc_test_static_run_case snprintf
        libc_test_static_run_case socket
        libc_test_static_run_case sscanf
        libc_test_static_run_case sscanf_long
        libc_test_static_run_case stat
        libc_test_static_run_case strftime
        libc_test_static_run_case string
        libc_test_static_run_case string_memcpy
        libc_test_static_run_case string_memmem
        libc_test_static_run_case string_memset
        libc_test_static_run_case string_strchr
        libc_test_static_run_case string_strcspn
        libc_test_static_run_case string_strstr
        libc_test_static_run_case strptime
        libc_test_static_run_case strtod
        libc_test_static_run_case strtod_simple
        libc_test_static_run_case strtof
        libc_test_static_run_case strtol
        libc_test_static_run_case strtold
        libc_test_static_run_case swprintf
        libc_test_static_run_case tgmath
        libc_test_static_run_case time
        libc_test_static_run_case tls_align
        libc_test_static_run_case udiv
        libc_test_static_run_case ungetc
        libc_test_static_run_case utime
        libc_test_static_run_case wcsstr
        libc_test_static_run_case wcstol
        libc_test_static_run_case daemon_failure
        libc_test_static_run_case dn_expand_empty
        libc_test_static_run_case dn_expand_ptr_0
        libc_test_static_run_case fflush_exit
        libc_test_static_run_case fgets_eof
        libc_test_static_run_case fgetwc_buffering
        libc_test_static_run_case fpclassify_invalid_ld80
        libc_test_static_run_case ftello_unflushed_append
        libc_test_static_run_case getpwnam_r_crash
        libc_test_static_run_case getpwnam_r_errno
        libc_test_static_run_case iconv_roundtrips
        libc_test_static_run_case inet_ntop_v4mapped
        libc_test_static_run_case inet_pton_empty_last_field
        libc_test_static_run_case iswspace_null
        libc_test_static_run_case lrand48_signextend
        libc_test_static_run_case lseek_large
        libc_test_static_run_case malloc_0
        libc_test_static_run_case mbsrtowcs_overflow
        libc_test_static_run_case memmem_oob_read
        libc_test_static_run_case memmem_oob
        libc_test_static_run_case mkdtemp_failure
        libc_test_static_run_case mkstemp_failure
        libc_test_static_run_case printf_1e9_oob
        libc_test_static_run_case printf_fmt_g_round
        libc_test_static_run_case printf_fmt_g_zeros
        libc_test_static_run_case printf_fmt_n
        libc_test_static_run_case pthread_robust_detach
        # libc_test_static_run_case pthread_cancel_sem_wait
        libc_test_static_run_case pthread_cond_smasher
        libc_test_static_run_case pthread_condattr_setclock
        libc_test_static_run_case pthread_exit_cancel
        libc_test_static_run_case pthread_once_deadlock
        # libc_test_static_run_case pthread_rwlock_ebusy
        libc_test_static_run_case putenv_doublefree
        libc_test_static_run_case regex_backref_0
        libc_test_static_run_case regex_bracket_icase
        libc_test_static_run_case regex_ere_backref
        libc_test_static_run_case regex_escaped_high_byte
        libc_test_static_run_case regex_negated_range
        libc_test_static_run_case regexec_nosub
        libc_test_static_run_case rewind_clear_error
        libc_test_static_run_case rlimit_open_files
        libc_test_static_run_case scanf_bytes_consumed
        libc_test_static_run_case scanf_match_literal_eof
        libc_test_static_run_case scanf_nullbyte_char
        libc_test_static_run_case setvbuf_unget
        libc_test_static_run_case sigprocmask_internal
        libc_test_static_run_case sscanf_eof
        libc_test_static_run_case statvfs
        libc_test_static_run_case strverscmp
        libc_test_static_run_case syscall_sign_extend
        libc_test_static_run_case uselocale_0
        libc_test_static_run_case wcsncpy_read_overflow
        libc_test_static_run_case wcsstr_false_negative
    fi
}

libc_test_static_run_case() {
    local case="$1"

    ./runtest.exe -w entry-static.exe "$case"
}

cyclic_test() {
    ./cyclictest_testcode.sh
}

lua_test() {
    ./lua_testcode.sh
}

ARCH="$1"
echo "Arch: $ARCH"

run_test "musl" basic_test "basic" true
run_test "glibc" basic_test  "basic" true
run_test "musl" busybox_test  "busybox" true
run_test "glibc" busybox_test "busybox" true
run_test "musl" lua_test
run_test "glibc" lua_test
run_test "musl" cyclic_test
run_test "glibc" cyclic_test
run_test "musl" libc_test "libctest" true "$ARCH"
run_test "glibc" libc_test "libctest" true "$ARCH"
