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

replace_in_file "README.md" "| English | [简体中文](./README.zh-cn.md) |" "| English | [简体中文](./README.md) |"
replace_in_file "README.zh-cn.md" "| [English](./README.md) | 简体中文 |" "| [English](./README.en-us.md) | 简体中文 |"

mv README.md README.en-us.md
mv README.zh-cn.md README.md
