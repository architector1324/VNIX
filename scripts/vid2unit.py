import cv2
import png
import gzip
import base64
import struct
import argparse

import numpy as np


def read_frame(cap):
    res, frame = cap.read()

    if not res:
        return None

    h, w, dim = frame.shape
    img = frame.flatten()
    dat = [tuple(reversed(img[n : n + 3])) for n in range(0, len(img), 3)]

    return (w, h, dim, dat)


def save_frame(filename, w, h, dim, dat):
    img = np.array([e for px in dat for e in px]).reshape((h, w * dim))

    img = png.from_array(img, 'RGB', {'width': w, 'height': h, 'bitdepth': 8})
    img.save(filename)


def diff_to_img(dat):
    return [unpack_diff(dpx)[1:4] for dpx in dat]


def pack_pixel(px):
    b = struct.pack('<BBB', px[0], px[1], px[2])
    return int.from_bytes(b, 'big')


def unpack_diff(diff):
    return tuple([np.uint8(e) for e in diff.to_bytes(4, 'big', signed=True)])


def zip_str(s):
    tmp0 = gzip.compress(bytes(s, 'utf-8'))
    tmp0 = base64.b64encode(tmp0).decode()

    tmp_s = gzip.compress(bytes(tmp0, 'utf-8'))
    tmp_s = base64.b64encode(tmp_s).decode()
    return f'`{tmp_s}`'


def zip_list(lst):
    lst0 = gzip.compress(bytes(lst))
    lst_s = base64.b64encode(lst0).decode()

    # print(f'zip: {len(lst)} -> {len(lst0)} -> {len(lst_s)}')

    return f'`{lst_s}`'


def rle_diff(dat):
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


def convert_img(size, dat, zip):
    img = [pack_pixel(px) for px in dat]
    # img_s = f'[{" ".join([str(e) for e in img])}]'

    if zip:
        # img_s = zip_str(img_s)
        img_s = zip_list(convert_to_bytes_img(img))
    else:
        img_s = f'[{" ".join([str(e) for e in img])}]'

    return f'{{size:({size[0]} {size[1]}) img:{img_s}}}'


def convert_int_to_bytes(v):
    if v == 0:
        lst = [13]
    elif -128 <= v <= 127:
        lst = [14]
        lst.extend(v.to_bytes(1, 'little', signed=True))
    elif 0 <= v <= 255:
        lst = [16]
        lst.extend(v.to_bytes(1, 'little', signed=False))
    elif -32768 <= v <= 32767:
        lst = [15]
        lst.extend(v.to_bytes(2, 'little', signed=True))
    elif 0 <= v <= 65535:
        lst = [17]
        lst.extend(v.to_bytes(2, 'little', signed=False))
    else: 
        lst = [3]
        lst.extend(v.to_bytes(4, 'little', signed=True))
    return lst


def convert_to_bytes_img(dat):
    lst = [11]
    lst.extend(len(dat).to_bytes(4, 'little', signed=False))

    for px in dat:
        lst.extend(convert_int_to_bytes(px))

    return lst


def convert_to_bytes_diff(map):
    lst = []

    lst.extend(len(map).to_bytes(2, 'little'))

    for pos in map:
        rle = map[pos]

        lst.extend(pos[0].to_bytes(2, 'little'))
        lst.extend(pos[1].to_bytes(2, 'little'))

        lst.extend(len(rle).to_bytes(2, 'little'))
        for cnt, dpx in rle:
            lst.extend(cnt.to_bytes(2, 'little'))
            lst.extend(dpx.to_bytes(4, 'little', signed=True))

    return lst


def convert_diff(size, diff, zip):
    map = {}

    for block_y in range(size[1] // 16):
        for block_x in range(size[0] // 16):
            tmp = []
            for y in range(16):
                for x in range(16):
                    idx = (x + block_x * 16) + (y + block_y * 16) * size[0]
                    tmp.append(diff[idx])
            tmp = rle_diff(tmp)

            if len(tmp) == 1 and tmp[0][1] == 0:
                continue
            map[(block_x, block_y)] = tmp

    if zip:
        lst_s = zip_list(convert_to_bytes_diff(map))
    else:
        raise Exception

    return lst_s


# parse args
parser = argparse.ArgumentParser()
parser.add_argument('vid', help='Video filename')
parser.add_argument('-z', '--zip', action='store_true', help='Compress video with gunzip')
parser.add_argument('-t', '--trc', action='store_true', help='Save codec trace output')

args = parser.parse_args()

# process video
cap = cv2.VideoCapture(args.vid)

# get first frame
(w, h, _, dat) = read_frame(cap)
img_s = convert_img((w, h), dat, args.zip)

# get next frame difference
frames_diff = []
last_frame = 0

while cap.isOpened():
    res = read_frame(cap)

    if res is None:
        break

    (_, _, _, next_dat) = res

    diff = [pack_pixel(next_dat[i]) - pack_pixel(dat[i]) for i in range(0, len(dat))]
    diff_s = convert_diff((w, h), diff, args.zip)
    dat = next_dat

    if args.trc:
        save_frame(f'./content/frames/out{last_frame}.png', w, h, 3, next_dat)
        save_frame(f'./content/frames/out{last_frame}d.png', w, h, 3, diff_to_img(diff))

    frames_diff.append(diff_s)
    last_frame += 1

frames_s = f'[{" ".join([s for s in frames_diff])}]'

# final
vid_s = f'{{img:{img_s} fms:{frames_s}}}'
print(vid_s)

cap.release()
