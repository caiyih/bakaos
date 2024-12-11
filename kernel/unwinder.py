import os
import sys
import re

def print_color(str, color):
    if color == 'red':
        print(f"\033[31m{str}\033[0m")
    elif color == 'green':
        print(f"\033[32m{str}\033[0m")
    elif color == 'yellow':
        print(f"\033[33m{str}\033[0m")
    elif color == 'blue':
        print(f"\033[34m{str}\033[0m")
    elif color == 'purple':
        print(f"\033[35m{str}\033[0m")
    elif color == 'cyan':
        print(f"\033[36m{str}\033[0m")
    elif color == 'white':
        print(f"\033[37m{str}\033[0m")
    else:
        print(str)

def read_pc_list(read_line):
    pc_list = []

    if not SILENT_MODE:
        print_color('Now, please provide stack trace info from the panicked kernel.', 'cyan')
        print_color('The lines between `Stack trace:` and `Note:` are needed. Include the two lines.', 'cyan')
        print_color('Lines not between the two lines will be ignored.', 'cyan')

    # Read from stdin and parse the PCs

    began = False

    while True:
        try:
            line = read_line()
        except EOFError:
            break

        if line == '':
            break

        if "Stack trace:" in line:
            began = True
            continue

        if "Note:" in line:
            break

        if not began:
            continue

        # Matching `at: 0x...`
        match = re.search(r'at:\s(0x[0-9a-fA-F]+)', line)
        if match:
            pc = int(match.group(1), 16)
            pc_list.append(pc)
        elif not SILENT_MODE:
            print_color(f"Invalid line: {line}", 'yellow')
    
    return pc_list

def build_symbol_table():
    mode = os.getenv('MODE', 'debug')
    os.system(f'make -C ../kernel symboltable MODE={mode}')

# read argument list to detect if has -slient or --silent

SILENT_MODE = False
FILE = None

if len(sys.argv) > 1:
    for arg in sys.argv:
        if arg == '-silent' or arg == '--silent':
            SILENT_MODE = True
        elif arg == '-f' or arg == '--file':
            FILE = sys.argv[sys.argv.index(arg) + 1]

if FILE is None:
    pc_list = read_pc_list(input)
else:
    # Read from file
    with open(FILE, 'r') as f:
        pc_list = read_pc_list(f.readline)

if len(pc_list) == 0 and SILENT_MODE:
    sys.exit(0)

build_symbol_table()

# 反汇编文件路径
disasm_file = '.disassembled'

try:
    # 读取反汇编文件
    with open(disasm_file, 'r') as f:
        lines = f.readlines()

    # 存储每个pc地址对应的信息
    pc_info = {}

    # 预处理反汇编内容，建立地址到行号的映射
    address_line_map = {}
    for idx, line in enumerate(lines):
        match = re.match(r'^([0-9a-f]+) <.*>', line)
        if match:
            address = int(match.group(1), 16)
            address_line_map[address] = idx

    print_color("Unwinding stack trace:", 'purple')

    # 遍历pc数组，获取详细信息
    for pc in pc_list:
        function_name = None
        source_file = None
        source_line = None
        start_idx_of_the_line = -1
        idx_if_method_name = -1

        if pc in address_line_map:
            idx = address_line_map[pc]
            # 向上查找函数名、源文件和行号
            for i in range(idx - 1, -1, -1):
                line = lines[i].strip()
                # 查找函数名
                if function_name is None and line.endswith(':'):
                    function_name = line[:-1] # Remove the trailing semicolon
                    idx_if_method_name = i
                    break
                # 查找源文件和行号
                if source_file is None:
                    source_match = re.match(r'^(.*):(\d+)', line)
                    if source_match:
                        source_file = source_match.group(1)
                        source_line = source_match.group(2)
                        start_idx_of_the_line = i + 1
                    continue

        print_color(f"  pc: {hex(pc)}", 'green')

        if start_idx_of_the_line == -1 and idx_if_method_name != -1:
            start_idx_of_the_line = idx_if_method_name + 1

        if start_idx_of_the_line != -1:
            if '()' not in function_name:
                function_name += '()'

            print_color(f"    at: {function_name} in {source_file}:{source_line}", 'yellow')
            print_color(f"    disassembly of the line:", 'blue')

            target_pc_idx = address_line_map[pc]
            i = start_idx_of_the_line
            while True:
                line = lines[i].strip()
                if line.endswith(':') or re.match(r'^(.*):(\d+)', line):
                    break

                if i == target_pc_idx:
                    print_color(f"      {line}", 'red')
                else:
                    print_color(f"      {line}", 'white')
                i += 1

            continue

        print_color('     Unwind info unavaliable', 'yellow')
finally:
    # Mark the CI as failed if there is any panic
    sys.exit(1)
