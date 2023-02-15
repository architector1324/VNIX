import cv2
import png
import gzip
import base64
import argparse
import numpy as np

import img2unit


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


def unpack_diff(diff):
    return tuple([np.uint8(e) for e in diff.to_bytes(4, 'big', signed=True)])


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


def convert_to_bytes_diff(map):
    lst = []

    lst.extend(len(map).to_bytes(2, 'little'))

    for pos in map:
        id = map[pos]

        lst.extend(pos[0].to_bytes(2, 'little'))
        lst.extend(pos[1].to_bytes(2, 'little'))

        lst.extend(id.to_bytes(3, 'little'))

    return lst


def convert_to_bytes_blocks(dat):
    lst = []

    lst.extend(len(dat).to_bytes(3, 'little'))

    for block in dat:
        lst.extend(len(block).to_bytes(2, 'little'))

        for cnt, px in block:
            lst.extend(cnt.to_bytes(2, 'little'))
            lst.extend(px.to_bytes(3, 'little'))

    return lst


def convert_to_bytes_colors(dat):
    lst = []

    lst.extend(len(dat).to_bytes(3, 'little'))

    for dpx in dat:
        lst.extend(dpx.to_bytes(4, 'little', signed=True))

    return lst

def convert_diff(size, diff, zip, diff_blocks):
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

            entry = tuple(tmp)

            if diff_blocks.get(entry) is None:
                diff_blocks[entry] = len(diff_blocks)

            diff_block_id = diff_blocks[entry]
            map[(block_x, block_y)] = diff_block_id

    if zip:
        lst_s = zip_list(convert_to_bytes_diff(map))
    else:
        lst_s = f'{{{" ".join(f"({key[0]} {key[1]}):{map[key]}" for key in map)}}}'

    return lst_s


def convert_blocks(map, colors, zip):
    lst = [k for k, _ in sorted(map.items(), key=lambda item: item[1])]
    lst = [[(cnt, colors[dpx]) for cnt, dpx in block] for block in lst]

    if zip:
        lst_s = zip_list(convert_to_bytes_blocks(lst))
    else:
        blocks_s = [f'[{" ".join(f"({cnt} {px})" for cnt, px in block)}]' for block in lst]
        lst_s = f'[{" ".join(s for s in blocks_s)}]'

    return lst_s


def convert_colors(map, zip):
    lst = [k for k, _ in sorted(map.items(), key=lambda item: item[1])]

    if zip:
        lst_s = zip_list(convert_to_bytes_colors(lst))
    else:
        lst_s = f'[{" ".join([str(e) for e in lst])}]'

    return lst_s

if __name__ == "__main__":
    # parse args
    parser = argparse.ArgumentParser()
    parser.add_argument('vid', help='Video filename')
    parser.add_argument('-z', '--zip', action='store_true', help='Compress video with gunzip')
    parser.add_argument('-t', '--trc', action='store_true', help='Save codec trace output')

    diff_blocks = {}
    diff_colors = {}

    args = parser.parse_args()

    # process video
    cap = cv2.VideoCapture(args.vid)

    # get first frame
    (w, h, _, dat) = read_frame(cap)
    img_s = img2unit.convert((w, h), dat, args.zip)

    # get next frame difference
    frames_diff = []
    last_frame = 0

    while cap.isOpened():
        res = read_frame(cap)

        if res is None:
            break

        (_, _, _, next_dat) = res

        diff = [img2unit.pack_pixel(next_dat[i]) - img2unit.pack_pixel(dat[i]) for i in range(0, len(dat))]

        for px in diff:
            if diff_colors.get(px) is None:
                diff_colors[px] = len(diff_colors)

        diff_s = convert_diff((w, h), diff, args.zip, diff_blocks)
        dat = next_dat

        if args.trc:
            save_frame(f'./content/frames/out{last_frame}.png', w, h, 3, next_dat)
            save_frame(f'./content/frames/out{last_frame}d.png', w, h, 3, diff_to_img(diff))

        frames_diff.append(diff_s)
        last_frame += 1

    blocks_s = convert_blocks(diff_blocks, diff_colors, args.zip)
    colors_s = convert_colors(diff_colors, args.zip)
    frames_s = f'[{" ".join([s for s in frames_diff])}]'

    # final
    vid_s = f'{{img:{img_s} pal:{colors_s} blk:{blocks_s} fms:{frames_s}}}'
    print(vid_s)

    cap.release()
