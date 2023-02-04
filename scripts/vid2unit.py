import cv2
import png
import gzip
import base64
import struct
import argparse


def read_frame(cap):
    res, frame = cap.read()

    if not res:
        return None

    h, w, dim = frame.shape
    img = frame.flatten()
    dat = [tuple(reversed(img[n : n + 3])) for n in range(0, len(img), 3)]

    return (w, h, dim, dat)


def pack_pixel(px):
    b = struct.pack('<BBB', px[0], px[1], px[2])
    return int.from_bytes(b, 'big')


def zip_str(s):
    tmp0 = gzip.compress(bytes(s, 'utf-8'))
    tmp0 = base64.b64encode(tmp0).decode()

    tmp_s = gzip.compress(bytes(tmp0, 'utf-8'))
    tmp_s = base64.b64encode(tmp_s).decode()
    return f'`{tmp_s}`'


def convert_img(size, dat, zip):
    img = []

    for px in dat:
        img.append(pack_pixel(px))

    img_s = f'[{" ".join([str(e) for e in img])}]'

    if zip:
        img_s = zip_str(img_s)

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
        lst_s = zip_str(lst_s)

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

while cap.isOpened():
    res = read_frame(cap)

    if res is None:
        break

    (_, _, _, next_dat) = res

    diff = [pack_pixel(next_dat[i]) - pack_pixel(dat[i]) for i in range(0, len(dat))]
    diff_s = convert_diff((w, h), diff, args.zip)
    dat = next_dat

    frames_diff.append(diff_s)

frames_s = f'[{" ".join([s for s in frames_diff])}]'
if args.zip:
    frames_s = zip_str(frames_s)

# final
vid_s = f'{{img:{img_s} fms:{frames_s}}}'
print(vid_s)

cap.release()
