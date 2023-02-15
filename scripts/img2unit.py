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


def rle_img(dat):
    lst = []

    cnt = 1
    prev = dat[0]

    for e in dat[1:]:
        if e != prev:
            lst.append((cnt, prev))

            prev = e
            cnt = 1
        else:
            cnt += 1

    lst.append((cnt, prev))

    return lst


def convert_to_bytes_rle(rle):
    lst = []

    for cnt, px in rle:
        lst.extend(cnt.to_bytes(3, 'little'))
        lst.extend(px.to_bytes(3, 'little'))

    return lst


def convert_to_bytes(dat):
    lst = []

    for px in dat:
        lst.extend(px.to_bytes(3, 'little'))

    return lst


def convert(size, dat, zip):
    img = [pack_pixel(px) for px in dat]
    img_rle = rle_img(img)

    if zip:
        img_b0 = convert_to_bytes(img)
        img_b1 = convert_to_bytes_rle(img_rle)

        img_b, fmt = (img_b0, 'rgb') if len(img_b0) < len(img_b1) else (img_b1, 'rgb.rle')

        # as binary representation
        img0 = gzip.compress(bytes(img_b))
        img_s = base64.b64encode(img0).decode()
        img_s = f'`{img_s}`'
    else:
        img_s0 = f'[{" ".join([str(e) for e in img])}]'
        img_s1 = f'[{" ".join(f"({cnt} {px})" for cnt, px in img_rle)}]'

        img_s, fmt = (img_s0, 'rgb') if len(img_s0) < len(img_s1) else (img_s1, 'rgb.rle')

    return f'{{size:({size[0]} {size[1]}) fmt:{fmt} img:{img_s}}}'


def convert_sys(size, dat):
    img = [e for px in dat for e in px]
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
