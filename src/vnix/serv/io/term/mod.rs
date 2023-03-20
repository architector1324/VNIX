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

use crate::driver::{DrvErr, CLIErr};
use crate::vnix::utils::Maybe;
use crate::vnix::core::task::ThreadAsync;

use crate::{thread, thread_await, as_async, maybe_ok, maybe, read_async, as_map_find_as_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServInfo, ServHlrAsync};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, UnitModify, DisplayStr};


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
    inp_lck: bool,
    font: &'static [(char, [u8; 16])],
    pub mode: Mode
}


impl Default for TermBase {
    fn default() -> Self {
        TermBase {
            pos: (0, 0),
            inp_lck: false,
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
                if ch != '\n' {
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
                }
            }
        };

        // move cursor
        if ch == '\n' {
            self.pos.0 = 0;
            self.pos.1 += 1;
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

fn cls(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (s, ath) = maybe!(as_async!(msg, as_str, ath, orig, kern));

        if s.as_str() != "cls" {
            return Ok(None)
        }

        let term = kern.lock().term.clone();

        term.lock().clear(kern).map_err(|e| KernErr::DrvErr(e))?;
        term.lock().flush(kern).map_err(|e| KernErr::DrvErr(e))?;

        Ok(Some(ath))
    })
}

fn nl(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (s, ath) = maybe!(as_async!(msg, as_str, ath, orig, kern));
    
        if s.as_str() != "nl" {
            return Ok(None)
        }

        let term = kern.lock().term.clone();

        term.lock().print_ch('\n', kern).map_err(|e| KernErr::DrvErr(e))?;
        term.lock().flush(kern).map_err(|e| KernErr::DrvErr(e))?;

        Ok(Some(ath))
    })
}

fn say(nl: bool, fmt:bool, ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (msg, mut ath) = maybe!(read_async!(msg, ath, orig, kern));

        if let Some(((s, msg), ath)) = as_async!(msg, as_pair, ath, orig, kern)? {
            let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));
            return match s.as_str() {
                // (say <unit>)
                "say" => thread_await!(say(false, false, ath, orig, msg, kern)),
                // (say.fmt [<unit> ..])
                "say.fmt" => thread_await!(say(false, true, ath, orig, msg, kern)),
                _ => Ok(None)
            }
        }

        // {say:<unit> nl:<t|f> shrt:<uint>}
        if let Some((_msg, mut ath)) = as_map_find_async!(msg, "say", ath, orig, kern)? {
            let nl = if let Some((nl, _ath)) = as_map_find_as_async!(msg, "nl", as_bool, ath, orig, kern)? {
                ath = _ath;
                nl
            } else {
                false
            };

            // FIXME: implement short
            let _shrt = if let Some((shrt, _ath)) = as_map_find_as_async!(msg, "shrt", as_uint, ath, orig, kern)? {
                ath = _ath;
                Some(shrt)
            } else {
                None
            };

            return thread_await!(say(nl, false, ath, orig, _msg, kern))
        }

        // {say.fmt:[<unit> ..] nl:<t|f> shrt:<uint>}
        if let Some((lst, mut ath)) = as_map_find_as_async!(msg, "say.fmt", as_list, ath, orig, kern)? {
            let nl = if let Some((nl, _ath)) = as_map_find_as_async!(msg, "nl", as_bool, ath, orig, kern)? {
                ath = _ath;
                nl
            } else {
                false
            };

            // FIXME: implement short
            let _shrt = if let Some((shrt, _ath)) = as_map_find_as_async!(msg, "shrt", as_uint, ath, orig, kern)? {
                ath = _ath;
                Some(shrt)
            } else {
                None
            };

            return thread_await!(say(nl, true, ath, orig, Unit::list_share(lst), kern))
        }

        // <unit>
        let mut s = if fmt {
            let (lst, _ath) = maybe!(as_async!(msg, as_list, ath, orig, kern));
            ath = _ath; 

            lst.iter().map(|u| format!("{}", DisplayStr(u.clone()))).collect()
        } else {
            format!("{}", DisplayStr(msg))
        };

        if nl {
            s += "\n";
        }

        let term = kern.lock().term.clone();

        term.lock().print(s.as_str(), kern).map_err(|e| KernErr::DrvErr(e))?;
        term.lock().flush(kern).map_err(|e| KernErr::DrvErr(e))?;

        Ok(Some(ath))
    })
}

pub fn term_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let mut ath = Rc::new(msg.ath.clone());

        // get command
        if let Some((res, ath)) = thread_await!(get(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), res)]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // cls command
        if let Some(_ath) = thread_await!(cls(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, msg.msg)?;
            }
            return Ok(Some(msg))
        }

        // nl command
        if let Some(_ath) = thread_await!(nl(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, msg.msg)?;
            }
            return Ok(Some(msg))
        }

        // say command
        if let Some(_ath) = thread_await!(say(false, false, ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, msg.msg)?;
            }
            return Ok(Some(msg))
        }

        Ok(Some(msg))
    })
}
