import argparse
import os
import lzma
import gzip
import shutil

def decompress_xz(compressed_file, output_file):
    with lzma.open(compressed_file) as f_in:
        with open(output_file, 'wb') as f_out:
            shutil.copyfileobj(f_in, f_out)

def decompress_gz(compressed_file, output_file):
    with gzip.open(compressed_file, 'rb') as f_in:
        with open(output_file, 'wb') as f_out:
            shutil.copyfileobj(f_in, f_out)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('img_file', help='The img file to decompress')
    args = parser.parse_args()

    img_file = args.img_file
    xz_file = f"{img_file}.xz"
    gz_file = f"{img_file}.gz"

    if os.path.exists(xz_file):
        print(f"Found .xz: {xz_file}")
        decompress_xz(xz_file, img_file)
        print(f"Decompressed {xz_file} to {img_file}")
    elif os.path.exists(gz_file):
        print(f"Found .gz: {gz_file}")
        decompress_gz(gz_file, img_file)
        print(f"Decompressed {gz_file} to {img_file}")
    else:
        print(f"No target image file was found: {img_file}.*")
        exit(1)

if __name__ == "__main__":
    main()
