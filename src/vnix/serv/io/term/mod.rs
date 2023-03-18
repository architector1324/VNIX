mod content;

use core::fmt::Display;

use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::format;
use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::DrvErr;
use crate::vnix::utils::Maybe;
use crate::vnix::core::task::ThreadAsync;

use crate::{thread, thread_await, as_async, maybe};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServInfo, ServHlrAsync};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI};


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
    
}

fn get(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Unit, Rc<String>), KernErr>> {
    thread!({
        let (s, ath) = maybe!(as_async!(msg, as_str, ath, orig, kern));

        let res = match s.as_str() {
            "get.mode" => {
                let mode = kern.lock().term.mode.clone();
                Unit::str(format!("{mode}").as_str())                
            },
            "get.res.txt" => {
                let (w, h) = kern.lock().drv.cli.res().map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?;
                Unit::pair(Unit::uint(w as u32), Unit::uint(h as u32))
            },
            "get.res.gfx" => {
                let (w, h) = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
                Unit::pair(Unit::uint(w as u32), Unit::uint(h as u32))
            },
            _ => return Ok(None)
        };
        Ok(Some((res, ath)))
    })
}

pub fn term_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());

        // get command
        if let Some((res, ath)) = thread_await!(get(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), res)]
            );
            writeln!(kern.lock().drv.cli, "{msg}");
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}
