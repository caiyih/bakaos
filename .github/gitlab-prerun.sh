#!/bin/bash

replace_in_file() {
    local file="$1"
    local search="$2"
    local replace="$3"
    
    if [ ! -f "$file" ]; then
        echo "错误: 文件 $file 不存在"
        return 1
    fi
    
    if sed -i.bak "s|${search}|${replace}|g" "$file"; then
        echo "已替换文件 $file 中的内容:"
        echo "  '$search' -> '$replace'"
        rm -f "$file.bak"
    else
        echo "错误: 替换文件 $file 失败"
        return 1
    fi
}

replace_in_file "README.md" "(./README.zh-cn.md)" "(./README.md)"
replace_in_file "README.zh-cn.md" "(./README.md)" "(./README.en-us.md)"

mv README.md README.en-us.md
mv README.zh-cn.md README.md
