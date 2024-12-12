#!/bin/bash

cargo build --release --target=x86_64-unknown-linux-musl
# cargo build --release --target=aarch64-unknown-linux-musl

if [ $? -ne 0 ]; then
    exit
fi

mkdir -p out

cp target/x86_64-unknown-linux-musl/release/vnix ./out/vnix_x86_64
cp content/vnix.store ./out/vnix.store

# dd if=/dev/zero of=./out/vnix.img bs=1048576 count=256
# 
# parted ./out/vnix.img -s -a minimal mklabel gpt
# parted ./out/vnix.img -s -a minimal mkpart EFI FAT32 2048s 93716s
# parted ./out/vnix.img -s -a minimal toggle 1 boot
# 
# mkfs.vfat ./out/vnix.img
# mmd -i ./out/vnix.img ::/EFI
# mmd -i ./out/vnix.img ::/EFI/BOOT
# mcopy -i ./out/vnix.img target/x86_64-unknown-uefi/release/vnix.efi ::/EFI/BOOT/BOOTX64.EFI
# mcopy -i ./out/vnix.img target/aarch64-unknown-uefi/release/vnix.efi ::/EFI/BOOT/BOOTAA64.EFI
# mcopy -i ./out/vnix.img content/vnix.store ::
# 
# poweriso -y convert out/vnix.img -o out/vnix.isos