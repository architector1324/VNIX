#!/bin/bash

cargo build --release --target=x86_64-unknown-linux-musl
# cargo build --release --target=aarch64-unknown-linux-musl

if [ $? -ne 0 ]; then
    exit
fi

mkdir -p bin
rm -f bin/*

cp target/x86_64-unknown-linux-musl/release/vnix-musl ./bin/vnix_x86_64
# cp target/aarch64-unknown-linux-musl/release/vnix-musl ./bin/vnix_aarch64

cp content/vnix.store ./bin/vnix.store

# dd if=/dev/zero of=./bin/vnix.img bs=1048576 count=256
# 
# parted ./bin/vnix.img -s -a minimal mklabel gpt
# parted ./bin/vnix.img -s -a minimal mkpart EFI FAT32 2048s 93716s
# parted ./bin/vnix.img -s -a minimal toggle 1 boot
# 
# mkfs.vfat ./bin/vnix.img
# mmd -i ./bin/vnix.img ::/EFI
# mmd -i ./bin/vnix.img ::/EFI/BOOT
# mcopy -i ./bin/vnix.img target/x86_64-unknown-uefi/release/vnix.efi ::/EFI/BOOT/BOOTX64.EFI
# mcopy -i ./bin/vnix.img target/aarch64-unknown-uefi/release/vnix.efi ::/EFI/BOOT/BOOTAA64.EFI
# mcopy -i ./bin/vnix.img content/vnix.store ::
# 
# poweriso -y convert bin/vnix.img -o bin/vnix.iso
