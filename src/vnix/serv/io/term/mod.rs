mod content;

use core::fmt::{Display, Write};

use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::format;
use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils::Maybe;
use crate::vnix::core::task::ThreadAsync;

use crate::{thread, thread_await, as_str_async, maybe};

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

fn get_mode(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Mode, Rc<String>), KernErr>> {
    thread!({
        let (s, ath) = maybe!(as_str_async!(msg, ath, orig, kern));

        match s.as_str() {
            "get.mode" => Ok(Some((kern.lock().term.mode.clone(), ath))),
            _ => Ok(None)
        }
    })
}

pub fn term_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());

        // get mode
        if let Some((mode, ath)) = thread_await!(get_mode(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::str(format!("{mode}").as_str()))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        yield;
        Ok(Some(msg))
    })
}
