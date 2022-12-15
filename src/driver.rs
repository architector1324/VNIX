pub mod vga {
    enum VGAConst {
        W = 25,
        H = 80
    }

    pub fn puts(s: &[u8]) {
        let vga_buf = 0xb8000 as *mut u8;

        for (i, b) in s.iter().enumerate() {
            unsafe {
                *vga_buf.offset(i as isize * 2) = *b;
                *vga_buf.offset(i as isize * 2 + 1) = 0xb;
            }
        }
    }
}
