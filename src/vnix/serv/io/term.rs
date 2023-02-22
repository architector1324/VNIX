use spin::Mutex;

use alloc::vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit};
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, Serv, ServHlr, ServHelpTopic};


#[derive(Debug, Clone)]
pub enum Mode {
    Cli,
    Gfx,
}

#[derive(Debug)]
pub struct TermBase {
    pos: (usize, usize)
}

#[derive(Debug)]
pub struct Term {
    mode: Mode,
}

impl Default for TermBase {
    fn default() -> Self {
        TermBase {
            pos: (0, 0)
        }
    }
}

impl Default for Term {
    fn default() -> Self {
        Term {
            mode: Mode::Cli,
        }
    }
}

impl FromUnit for Term {
    fn from_unit_loc(_u: &Unit) -> Option<Self> {
        let term = Term::default();
        Some(term)
    }
}

impl ServHlr for Term {
    fn help<'a>(self, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Terminal I/O service\nExample: hello@io.term\nFor gfx mode: {term.gfx:(say hello)}@io.term".into())
            };
    
            let m = Unit::Map(vec![(
                Unit::Str("msg".into()),
                u
            )]);
    
            let out = kern.lock().msg(&ath, m).map(|msg| Some(msg));
            yield;

            out
        };
        ServHlrAsync(Box::new(hlr))
    }

    fn handle<'a>(mut self, msg: Msg, _serv: Serv, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            writeln!(kern.lock().drv.cli, "io.term: ok").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            yield;

            Ok(Some(msg))
        };
        ServHlrAsync(Box::new(hlr))
    }
}