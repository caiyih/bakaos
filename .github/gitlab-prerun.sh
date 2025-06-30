#!/bin/bash

replace_in_file() {
    local file="$1"
    local search="$2"
    local replace="$3"

    if [[ ! -f "$file" ]]; then
        echo "Error: File '$file' does not exist."
        return 1
    fi

    sed -i "s|$search|$replace|g" "$file"
}

replace_in_file "README.md" "(./README.zh-cn.md)" "(./README.md)"
replace_in_file "README.zh-cn.md" "(./README.md)" "(./README.en-us.md)"

mv README.md README.en-us.md
mv README.zh-cn.md README.md
