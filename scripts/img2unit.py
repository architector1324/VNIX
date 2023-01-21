import png
import gzip
import base64
import struct
import argparse


def read_img(filename):
    img = png.Reader(filename).read_flat()

    w = img[0]
    h = img[1]
    dim = img[3]['planes']

    if dim == 3:
        dat = [tuple(img[2][n : n + 3]) for n in range(0, len(img[2]), 3)]
    elif dim == 4:
        dat = [tuple(img[2][n : n + 4]) for n in range(0, len(img[2]), 4)]
    else:
        raise Exception

    return ((w, h), dim, dat)


def convert(size, dim, dat, zip):
    img = []

    for px in dat:
        b = struct.pack('<BBB', px[0], px[1], px[2])
        v = int.from_bytes(b, 'big')
        img.append(v)

    img_s = f'[{" ".join([str(e) for e in img])}]'

    if zip:
        img0 = gzip.compress(bytes(img_s, 'utf-8'))
        img0 = base64.b64encode(img0).decode()

        img_s = gzip.compress(bytes(img0, 'utf-8'))
        img_s = base64.b64encode(img_s).decode()
        img_s = f'`{img_s}`'

    return f'{{img:(({size[0]} {size[1]}) {img_s})}}'


# parse args
parser = argparse.ArgumentParser()
parser.add_argument('img', help='Image filename')
parser.add_argument('-z', '--zip', action='store_true', help='Compress image with gunzip')

args = parser.parse_args()

# process image
(size, dim, img) = read_img(args.img)
vnix_img = convert(size, dim, img, args.zip)

print(vnix_img)
