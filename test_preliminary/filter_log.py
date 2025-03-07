import re
import sys

def filter_log(input_path, output_path):
    pattern = re.compile(r"(#### OS COMP TEST GROUP START basic-.*? ####)(.*?)(#### OS COMP TEST GROUP END basic-.*? ####)", re.S)
    with open(input_path, 'r', encoding='utf-8') as infile, open(output_path, 'w', encoding='utf-8') as outfile:
        content = infile.read()
        matches = pattern.findall(content)
        for start, body, end in matches:
            outfile.write(start + "\n")
            outfile.write(body.strip() + "\n")
            outfile.write(end + "\n\n")
        print(f"Filtered log saved to {output_path}")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python3 filter_log.py input.log output.log")
        sys.exit(1)
    filter_log(sys.argv[1], sys.argv[2])