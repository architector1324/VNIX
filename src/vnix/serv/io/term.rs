use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::sync::Arc;

use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaStr, Schema, SchemaMapEntry, SchemaUnit, SchemaOr, SchemaSeq, Or};
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, Serv, ServHlr, ServHelpTopic};


#[derive(Debug, Clone)]
enum Mode {
    Cli,
    Gfx,
}

#[derive(Debug)]
pub struct TermBase {
    pos: (usize, usize)
}

pub struct TermActAsync<'a>(Box<dyn Generator<Yield = (), Return = Result<Unit, KernErr>> + 'a>);

pub trait TermAct {
    fn act<'a>(self, orig: Arc<Msg>, msg: Unit, term: Arc<Mutex<Term>>, kern: &'a Mutex<Kern>) -> TermActAsync<'a>;
}

#[derive(Debug, Clone)]
enum Act {
    Cls,
}

#[derive(Debug)]
pub struct Term {
    mode: Mode,
    acts: Option<Vec<Act>>
}

impl Term {
    fn clear(&mut self, kern: &mut Kern) -> Result<(), CLIErr> {
        match self.mode {
            Mode::Cli => kern.drv.cli.clear()?,
            Mode::Gfx => todo!()
        }
        kern.term.pos = (0, 0);

        Ok(())
    }
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
            acts: None
        }
    }
}

impl FromUnit for Act {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaStr;

        schm.find_deep(glob, u).and_then(|s| {
            match s.as_str() {
                "cls" => Some(Act::Cls),
                _ => None
            }
        })
    }
}

impl FromUnit for Term {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut term = Term::default();

        let schm = SchemaMapEntry(
            Unit::Str("term".into()),
            SchemaOr(
                SchemaSeq(SchemaUnit),
                SchemaUnit
            )
        );

        term.acts = schm.find_loc(u).and_then(|or| {
            match or {
                Or::First(seq) => Some(seq.into_iter().map(|act| Act::from_unit(&u, &act)).collect::<Option<Vec<_>>>()?),
                Or::Second(act) => Some(vec![Act::from_unit(u, &act)?])
            }
        });

        Some(term)
    }
}

impl TermAct for Act {
    fn act<'a>(self, _orig: Arc<Msg>, msg: Unit, term: Arc<Mutex<Term>>, kern: &'a Mutex<Kern>) -> TermActAsync<'a> {
        let hlr = match self {
            Act::Cls => move || {
                term.lock().clear(&mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                yield;

                Ok(msg)
            }
        };
        TermActAsync(Box::new(hlr))
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

    fn handle<'a>(self, msg: Msg, _serv: Serv, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            writeln!(kern.lock().drv.cli, "io.term: {:?}", self.acts);

            if let Some(acts) = self.acts.clone() {
                let mut out_u = msg.msg.clone();

                let term = Arc::new(Mutex::new(self));
                let msg = Arc::new(msg);

                for act in acts {
                    let mut gen = Box::into_pin(act.act(msg.clone(), out_u.clone(), term.clone(), kern).0);

                    loop {
                        if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
                            out_u = out_u.merge(res?);
                            break;
                        }
                        yield;
                    }
                    yield;
                }

                let msg = kern.lock().msg(&msg.ath, out_u)?;
                return Ok(Some(msg))
            }

            Ok(Some(msg))
        };
        ServHlrAsync(Box::new(hlr))
    }
}