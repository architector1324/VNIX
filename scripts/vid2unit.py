import cv2
import png
import gzip
import base64
import struct
import argparse


def read_frame(cap):
    _, frame = cap.read()

    h, w, dim = frame.shape
    img = frame.flatten()
    dat = [tuple(reversed(img[n : n + 3])) for n in range(0, len(img), 3)]

    return (w, h, dim, dat)


def pack_pixel(px):
    b = struct.pack('<BBB', px[0], px[1], px[2])
    return int.from_bytes(b, 'big')


def convert_img(size, dat, zip):
    img = []

    for px in dat:
        img.append(pack_pixel(px))

    img_s = f'[{" ".join([str(e) for e in img])}]'

    if zip:
        img0 = gzip.compress(bytes(img_s, 'utf-8'))
        img0 = base64.b64encode(img0).decode()

        img_s = gzip.compress(bytes(img0, 'utf-8'))
        img_s = base64.b64encode(img_s).decode()
        img_s = f'`{img_s}`'

    return f'{{size:({size[0]} {size[1]}) img:{img_s}}}'


def convert_diff(size, diff, zip):
    lst = []
    for i, dpx in enumerate(diff):
        if dpx != 0:
            x = i % size[0]
            y = i // size[0]
            lst.append(((x, y), dpx))

    lst_s = f'[{" ".join([f"(({x} {y}) {dpx})" for ((x, y), dpx) in lst])}]'

    if zip:
        lst0 = gzip.compress(bytes(lst_s, 'utf-8'))
        lst0 = base64.b64encode(lst0).decode()

        lst_s = gzip.compress(bytes(lst0, 'utf-8'))
        lst_s = base64.b64encode(lst_s).decode()
        lst_s = f'`{lst_s}`'

    return lst_s
     

# parse args
parser = argparse.ArgumentParser()
parser.add_argument('vid', help='Video filename')
parser.add_argument('-z', '--zip', action='store_true', help='Compress video with gunzip')

args = parser.parse_args()

# process video
cap = cv2.VideoCapture(args.vid)

# get first frame
(w, h, _, dat) = read_frame(cap)
img_s = convert_img((w, h), dat, args.zip)

# get next frame difference
frames_diff = []

for _ in range(120):
    (_, _, _, next_dat) = read_frame(cap)
    diff = [pack_pixel(next_dat[i]) - pack_pixel(dat[i]) for i in range(0, len(dat))]
    diff_s = convert_diff((w, h), diff, args.zip)
    dat = next_dat

    frames_diff.append(diff_s)

frames_s = f'[{" ".join([s for s in frames_diff])}]'

# final
vid_s = f'{{img:{img_s} fms:{frames_s}}}'
print(vid_s)

cap.release()
