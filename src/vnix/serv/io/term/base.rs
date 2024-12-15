use spin::Mutex;

use alloc::rc::Rc;
use alloc::string::String;

use crate::vnix::utils::Maybe;

use crate::vnix::core::task::Yield;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::unit::{Unit, UnitNew};
use crate::vnix::core::driver::{DrvErr, CLIErr, TermKey};


#[derive(Debug)]
pub struct Term {
    pos: (usize, usize),
    font: &'static [(char, [u8; 16])],
    pub mode: super::Mode
}

impl Term {
    pub fn new(font: &'static [(char, [u8; 16])]) -> Term {
        Term {
            pos: (0, 0),
            font: font,
            mode: super::Mode::Gfx
        }
    }

    pub fn clear(&mut self, kern: &mut Kern) -> Result<(), DrvErr> {
        self.pos = (0, 0);

        match self.mode {
            super::Mode::Text => kern.drv.cli.clear().map_err(|e| DrvErr::CLI(e)),
            super::Mode::Gfx => kern.drv.disp.fill(&|_, _| 0).map_err(|e| DrvErr::Disp(e)),
        }
    }

    pub fn flush(&mut self, kern: &mut Kern) -> Result<(), DrvErr> {
        match self.mode {
            super::Mode::Gfx => kern.drv.disp.flush().map_err(|e| DrvErr::Disp(e)),
            _ => Ok(())
        }
    }

    pub fn print_ch(&mut self, ch: char, kern: &mut Kern) -> Result<(), DrvErr> {
        let (w, h) = match self.mode {
            super::Mode::Text => kern.drv.cli.res().map_err(|e| DrvErr::CLI(e))?,
            super::Mode::Gfx => {
                let res = kern.drv.disp.res().map_err(|e| DrvErr::Disp(e))?;
                (res.0 / 8, res.1 / 16)
            }
        };

        // display char
        match self.mode {
            super::Mode::Text => write!(kern.drv.cli, "{ch}").map_err(|_| DrvErr::CLI(CLIErr::Write))?,
            super::Mode::Gfx => {
                if ch == '\u{8}' {
                    for y in 0..16 {
                        for x in 0..8 {
                            kern.drv.disp.px(0, x + (self.pos.0 - 1) * 8, y + self.pos.1 * 16).map_err(|e| DrvErr::Disp(e))?;
                        }
                    }
                    kern.drv.disp.flush_blk(((self.pos.0 - 1) as i32 * 8, self.pos.1 as i32 * 16), (8, 16)).map_err(|e| DrvErr::Disp(e))?;
                } else if !(ch == '\n' || ch == '\r') {
                    let img = self.font.iter().find_map(|(_ch, img)| {
                        if *_ch == ch {
                            return Some(img)
                        }
                        None
                    }).ok_or(DrvErr::CLI(CLIErr::Write))?;
    
                    for y in 0..16 {
                        for x in 0..8 {
                            let px = if (img[y] >> (8 - x)) & 1 == 1 {0xffffff} else {0};
                            kern.drv.disp.px(px, x + self.pos.0 * 8, y + self.pos.1 * 16).map_err(|e| DrvErr::Disp(e))?;
                        }
                    }
                    kern.drv.disp.flush_blk((self.pos.0 as i32 * 8, self.pos.1 as i32 * 16), (8, 16)).map_err(|e| DrvErr::Disp(e))?;
                }
            }
        };

        // move cursor
        if ch == '\n' || ch == '\r' {
            self.pos.0 = 0;
            self.pos.1 += 1;
        } else if ch == '\u{8}' && self.pos.0 != 0 {
            self.pos.0 -= 1;
        } else {
            self.pos.0 += 1;
            if self.pos.0 == w {
                self.pos.0 = 0;
                self.pos.1 += 1;
            }
        }

        // workaround for scrolling in gfx mode
        if let super::Mode::Gfx = self.mode {
            if self.pos.1 >= h {
                self.clear(kern)?;
                self.flush(kern)?;
            }
        }
        Ok(())
    }

    pub fn print(&mut self, s: &str, kern: &mut Kern) -> Result<(), DrvErr> {
        for ch in s.chars() {
            self.print_ch(ch, kern)?;
        }
        Ok(())
    }

    pub async fn input(term: Rc<Mutex<Self>>, secret:bool, limit: Option<usize>, kern: &Mutex<Kern>) -> Maybe<Unit, KernErr> {
        let save_pos = term.lock().pos.clone();

        let mut s = String::new();
        loop {
            // get key
            let mut grd = kern.lock();
            let key = grd.drv.cli.get_key(false).map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?;
            drop(grd);

            // push to string
            if let Some(key) = key {
                match key {
                    TermKey::Char(ch) => {
                        if ch == '\n' || ch == '\r' {
                            break;
                        }

                        if ch == '\u{8}' {
                            if term.lock().pos.0 > save_pos.0 {
                                s.pop();
                                if !secret {
                                    term.lock().print_ch(ch, &mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;
                                }
                            }

                            Yield::now().await;
                            continue;
                        }

                        if let Some(lim) = limit {
                            if s.len() >= lim {
                                Yield::now().await;
                                continue;
                            }
                        }

                        if ch.is_control() {
                            Yield::now().await;
                            continue;
                        }

                        s.push(ch);
                        if !secret {
                            term.lock().print_ch(ch, &mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;
                        }
                    },
                    TermKey::Esc => break,
                    _ => Yield::now().await
                }
            }
            Yield::now().await;
        }

        if s.is_empty() {
            return Ok(None)
        }
        return Ok(Some(Unit::str(&s)))
    }
}
