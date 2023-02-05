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

    # print(len(img[2]))

    if dim == 3:
        dat = [tuple(img[2][n : n + 3]) for n in range(0, len(img[2]), 3)]
    elif dim == 4:
        dat = [tuple(img[2][n : n + 4]) for n in range(0, len(img[2]), 4)]
    else:
        raise Exception

    return ((w, h), dim, dat)


def pack_pixel(px):
    b = struct.pack('<BBB', px[0], px[1], px[2])
    return int.from_bytes(b, 'big')


def convert_to_bytes(dat):
    lst = [8]
    lst.extend(len(dat).to_bytes(4, 'little', signed=False))

    for px in dat:
        lst.append(3)
        lst.extend(px.to_bytes(4, 'little', signed=True))

    return lst


def convert(size, dat, zip):
    img = [pack_pixel(px) for px in dat]

    # img_b = convert_to_bytes(img)

    # print(len(img_s), len(img_b))

    if zip:
        # as binary representation
        img_b = convert_to_bytes(img)
        img0 = gzip.compress(bytes(img_b))
        img_s = base64.b64encode(img0).decode()
        img_s = f'`{img_s}`'

        # # as string representation
        # img0 = gzip.compress(bytes(img_s, 'utf-8'))
        # img0 = base64.b64encode(img0).decode()

        # img_s = gzip.compress(bytes(img0, 'utf-8'))
        # img_s = base64.b64encode(img_s).decode()
        # img_s = f'`{img_s}`'
    else:
        img_s = f'[{" ".join([str(e) for e in img])}]'

    return f'{{size:({size[0]} {size[1]}) img:{img_s}}}'


def convert_sys(size, dat):
    img = []

    for px in dat:
        b = struct.pack('<BBB', px[0], px[1], px[2])
        v = int.from_bytes(b, 'big')
        img.append(v)

    img_s = f'[{",".join([str(e) for e in img])}]'

    return (img_s, len(img))


# parse args
parser = argparse.ArgumentParser()
parser.add_argument('img', help='Image filename')
parser.add_argument('--sys', action='store_true', help='Convert to system img array')
parser.add_argument('-z', '--zip', action='store_true', help='Compress image with gunzip')

args = parser.parse_args()

# process image
(size, _, img) = read_img(args.img)

if args.sys:
    vnix_img = convert_sys(size, img)
else:
    vnix_img = convert(size, img, args.zip)

print(vnix_img)
