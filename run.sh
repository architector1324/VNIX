#!/bin/bash

qemu-system-x86_64 -enable-kvm -m 1024M -full-screen -serial stdio -vga virtio -device virtio-rng-pci \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/ovmf/x64/OVMF.fd \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/ovmf/x64/OVMF_VARS.fd \
    -cdrom ./out/vnix.img
