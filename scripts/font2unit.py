from bdflib import reader
import argparse


# parse args
parser = argparse.ArgumentParser()
parser.add_argument('font', help='Font filename')
parser.add_argument('--sys', action='store_true', help='Convert to system font array')
parser.add_argument('--ascii', action='store_true', help='Use only ascii with some vnix needed symbols')

args = parser.parse_args()

# load font
ascii = ['λ', ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?', '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_', '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~']

replace = {'`': "'`'", '\\': "`\\\\`"}

with open(args.font, 'rb') as hlr:
    font = reader.read_bdf(hlr)

    if args.sys:
        for ch in font.glyphs_by_codepoint:
            dat = font[ch].data
            dat.reverse()

            ch = chr(ch) if ch != "'" else "\\'"

            if args.ascii and not ch in ascii:
                continue

            print(f"('{ch}', {dat}),")

        print(len(font.glyphs_by_codepoint))
    else:
        glyths = []

        for ch in font.glyphs_by_codepoint:
            dat = font[ch].data
            dat.reverse()

            dat_s = f'[{" ".join(["0x{0:02x}".format(e) for e in dat])}]'

            if args.ascii and not chr(ch) in ascii:
                continue

            ch = f"`{chr(ch)}`" if not chr(ch) in replace else replace[chr(ch)]
            glyths.append(f"{ch}:{dat_s}")

        glyths_s = f'{" ".join([str(e) for e in glyths])}'

        print(f'{{font:{{{glyths_s}}}}}')
