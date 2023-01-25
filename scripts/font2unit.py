from bdflib import reader
import argparse


# parse args
parser = argparse.ArgumentParser()
parser.add_argument('font', help='Font filename')

args = parser.parse_args()

# load font
chars = ['λ', ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?', '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_', '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~']

with open(args.font, 'rb') as hlr:
    font = reader.read_bdf(hlr)

    for ch in chars:
        dat = font[ord(ch)].data
        dat.reverse()

        print(f"('{ch}', {dat}),")

    print(len(chars))
