pub mod amd64 {
    pub mod vga {
        const BUF_W: usize = 80;
        const BUF_H: usize = 25;
    
        #[derive(Clone, Copy)]
        #[repr(C)]
        struct Char {
            ch: u8,
            attr: u8
        }
    
        pub struct Buf <'a>{
            pos: usize,
            buf: &'a mut [[volatile::Volatile<Char>; BUF_W]; BUF_H]
        }
    
        impl Buf<'_> {
            pub fn put(&mut self, ch: u8) {
                self.buf[self.pos / BUF_W][self.pos % BUF_W].write(Char{ch: ch, attr: 0xf});
                self.pos += 1;
            }
    
            pub fn puts(&mut self, s: &str) {
                s.bytes().for_each(|b| self.put(b));
            }
        }
    
        impl core::fmt::Write for Buf<'_> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.puts(s);
                Ok(())
            }
        }
    
        impl Default for Buf<'_> {
            fn default() -> Self {
                Buf {
                    pos: 0,
                    buf: unsafe {&mut *(0xb8000 as *mut [[volatile::Volatile<Char>; BUF_W]; BUF_H])},
                }
            }
        }
    }    
}
