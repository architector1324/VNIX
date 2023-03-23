mod text;
mod content;

use core::fmt::Display;

use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::format;
use alloc::vec::Vec;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::{DrvErr, CLIErr, TermKey};
use crate::vnix::utils::Maybe;
use crate::vnix::core::task::ThreadAsync;

use crate::{thread, thread_await, as_async, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServInfo, ServHlrAsync};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, UnitModify, UnitParse};


pub const SERV_PATH: &'static str = "io.term";
pub const SERV_HELP: &'static str = "Terminal I/O service\nExample: hello@io.term";


#[derive(Debug, Clone)]
pub enum Mode {
    Text,
    Gfx,
}

#[derive(Debug)]
pub struct TermBase {
    pos: (usize, usize),
    font: &'static [(char, [u8; 16])],
    pub mode: Mode
}


impl Default for TermBase {
    fn default() -> Self {
        TermBase {
            pos: (0, 0),
            font: &content::SYS_FONT,
            mode: Mode::Gfx
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Mode::Text => write!(f, "txt"),
            Mode::Gfx => write!(f, "gfx")
        }
    }
}

impl TermBase {
    fn clear(&mut self, kern: &Mutex<Kern>) -> Result<(), DrvErr> {
        self.pos = (0, 0);

        match self.mode {
            Mode::Text => kern.lock().drv.cli.clear().map_err(|e| DrvErr::CLI(e)),
            Mode::Gfx => kern.lock().drv.disp.fill(&|_, _| 0).map_err(|e| DrvErr::Disp(e)),
        }
    }

    fn flush(&mut self, kern: &Mutex<Kern>) -> Result<(), DrvErr> {
        match self.mode {
            Mode::Gfx => kern.lock().drv.disp.flush().map_err(|e| DrvErr::Disp(e)),
            _ => Ok(())
        }
    }

    fn print_ch(&mut self, ch: char, kern: &Mutex<Kern>) -> Result<(), DrvErr> {
        let (w, _) = kern.lock().drv.cli.res().map_err(|e| DrvErr::CLI(e))?;

        // display char
        match self.mode {
            Mode::Text => write!(kern.lock().drv.cli, "{ch}").map_err(|_| DrvErr::CLI(CLIErr::Write))?,
            Mode::Gfx => {
                if ch == '\u{8}' {
                    for y in 0..16 {
                        for x in 0..8 {
                            kern.lock().drv.disp.px(0, x + (self.pos.0 - 1) * 8, y + self.pos.1 * 16).map_err(|e| DrvErr::Disp(e))?;
                        }
                    }
                    kern.lock().drv.disp.flush_blk(((self.pos.0 - 1) as i32 * 8, self.pos.1 as i32 * 16), (8, 16)).map_err(|e| DrvErr::Disp(e))?;
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
                            kern.lock().drv.disp.px(px, x + self.pos.0 * 8, y + self.pos.1 * 16).map_err(|e| DrvErr::Disp(e))?;
                        }
                    }
                    kern.lock().drv.disp.flush_blk((self.pos.0 as i32 * 8, self.pos.1 as i32 * 16), (8, 16)).map_err(|e| DrvErr::Disp(e))?;
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
        Ok(())
    }

    fn print(&mut self, s: &str, kern: &Mutex<Kern>) -> Result<(), DrvErr> {
        for ch in s.chars() {
            self.print_ch(ch, kern)?;
        }
        Ok(())
    }

    fn input(term: Rc<Mutex<Self>>, secret:bool, parse: bool, limit: Option<usize>, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Unit, KernErr>> {
        thread!({
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
                                        term.lock().print_ch(ch, kern).map_err(|e| KernErr::DrvErr(e))?;
                                    }
                                }

                                yield;
                                continue;
                            }

                            if let Some(lim) = limit {
                                if s.len() >= lim {
                                    yield;
                                    continue;
                                }
                            }

                            s.push(ch);
                            if !secret {
                                term.lock().print_ch(ch, kern).map_err(|e| KernErr::DrvErr(e))?;
                            }
                        },
                        TermKey::Esc => break,
                        _ => yield
                    }
                }
                yield;
            }

            if s.is_empty() {
                return Ok(None)
            }

            // parse string
            if parse {
                let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
                return Ok(Some(u))
            }
            return Ok(Some(Unit::str(&s)))
        })
    }
}

fn get(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Unit, Rc<String>), KernErr>> {
    thread!({
        let info = {
            let mode = kern.lock().term.lock().mode.clone();
            let res_txt = kern.lock().drv.cli.res().map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?;
            let res_gfx = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;

            let res_all_txt = kern.lock().drv.cli.res_list()
                .map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?
                .into_iter()
                .map(|(w, h)| Unit::pair(Unit::uint(w as u32), Unit::uint(h as u32)))
                .collect::<Vec<_>>();

            let res_all_gfx = kern.lock().drv.disp.res_list()
                .map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?
                .into_iter()
                .map(|(w, h)| Unit::pair(Unit::uint(w as u32), Unit::uint(h as u32)))
                .collect::<Vec<_>>();

            Unit::map(&[
                (
                    Unit::str("mode"),
                    Unit::str(format!("{mode}").as_str())
                ),
                (
                    Unit::str("res"),
                    Unit::map(&[
                        (
                            Unit::str("txt"),
                            Unit::pair(
                                Unit::uint(res_txt.0 as u32),
                                Unit::uint(res_txt.1 as u32)
                            )
                        ),
                        (
                            Unit::str("gfx"),
                            Unit::pair(
                                Unit::uint(res_gfx.0 as u32),
                                Unit::uint(res_gfx.1 as u32)
                            )
                        ),
                        (
                            Unit::str("all"),
                            Unit::map(&[
                                (
                                    Unit::str("txt"),
                                    Unit::list(&res_all_txt)
                                ),
                                (
                                    Unit::str("gfx"),
                                    Unit::list(&res_all_gfx)
                                )
                            ])
                        )
                    ])
                )
            ])
        };

        // get
        if let Some((s, ath)) = as_async!(msg, as_str, ath, orig, kern)? {
            let res = match s.as_str() {
                "get" => info,
                _ => return Ok(None)
            };
            return Ok(Some((res, ath)))
        }

        // get with ref
        if let Some(path) = msg.as_path() {
            let mut path = path.iter().map(|s| s.as_str());

            let res = match maybe_ok!(path.next()) {
                "get" => maybe_ok!(info.find(path)),
                _ => return Ok(None)
            };
            return Ok(Some((res, ath)))
        }
        Ok(None)
    })
}

pub fn term_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let mut ath = Rc::new(msg.ath.clone());

        // get command
        if let Some((msg, ath)) = thread_await!(get(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // cls command
        if let Some(_ath) = thread_await!(text::cls(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, msg.msg)?;
            }
            return Ok(Some(msg))
        }

        // nl command
        if let Some(_ath) = thread_await!(text::nl(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, msg.msg)?;
            }
            return Ok(Some(msg))
        }

        // get key command
        if let Some((key, ath)) = thread_await!(text::get_key(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::str(format!("{key}").as_str()))
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // input command
        if let Some((_msg, ath)) = thread_await!(text::input(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            if let Some(msg) = _msg {
                let msg = Unit::map(&[
                    (Unit::str("msg"), msg)
                ]);
                return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
            }
            return Ok(Some(msg))
        }

        // say command
        if let Some(_ath) = thread_await!(text::say(false, false, ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, msg.msg)?;
            }
            return Ok(Some(msg))
        }
        Ok(Some(msg))
    })
}
