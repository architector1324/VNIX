from bdflib import reader
import argparse


# parse args
parser = argparse.ArgumentParser()
parser.add_argument('font', help='Font filename')

args = parser.parse_args()

# load font
with open(args.font, 'rb') as hlr:
    font = reader.read_bdf(hlr)

    for ch in font.glyphs_by_codepoint:
        dat = font[ch].data
        dat.reverse()

        print(f"('{chr(ch)}', {dat}),")

    print(len(font.glyphs_by_codepoint))
