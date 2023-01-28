mod content;

use core::fmt::Write;

use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::string::String;
use alloc::vec::Vec;
use sha2::digest::typenum::Mod;

use crate::driver::{CLIErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, DisplayShort, SchemaMapSecondRequire, SchemaMapEntry, SchemaBool, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMap, SchemaPair, SchemaOr, SchemaSeq, Or, SchemaMapRequire, SchemaMapSeq, SchemaByte, SchemaRef};

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::utils;


#[derive(Debug, Clone)]
pub enum Act {
    Clear,
    Nl,
    GetKey,
    Trc,
    Say(Unit)
}

#[derive(Debug)]
pub enum Mode {
    Cli,
    Gfx,
}

#[derive(Debug)]
pub struct Term {
    mode: Mode,
    act: Option<Vec<Act>>
}

impl Act {
    fn act(self, term: &mut Term, msg: &Msg, kern: &mut Kern) -> Result<(), KernErr> {
        match self {
            Act::Clear => term.clear(kern),
            Act::Nl => term.print("\n", kern),
            Act::Trc => term.print(format!("{}", msg).as_str(), kern),
            Act::Say(msg) => term.out(&msg, kern),
            Act::GetKey => term.get_key(kern)
        }
    }
}

impl Term {
    fn print(&mut self, s: &str, kern: &mut Kern) -> Result<(), KernErr> {
        match self.mode {
            Mode::Cli => write!(kern.cli, "{s}").map_err(|_| KernErr::CLIErr(CLIErr::Write)),
            Mode::Gfx => todo!()
        }
    }

    fn clear(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        match self.mode {
            Mode::Cli => kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear)),
            Mode::Gfx => todo!()
        }
    }

    fn get_key(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        let _key = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
        Ok(())
    }

    fn out(&mut self, msg: &Unit, kern: &mut Kern) -> Result<(), KernErr> {
        match self.mode {
            Mode::Cli => write!(kern.cli, "{}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write)),
            Mode::Gfx => todo!()
        }
    }
}

impl Default for Term {
    fn default() -> Self {
        Term {
            mode: Mode::Cli,
            act: None
        }
    }
}

impl FromUnit for Act {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaStr,
            SchemaPair(SchemaStr, SchemaUnit)
        );

        schm.find(glob, u).and_then(|or| {
            match or {
                Or::First(s) =>
                match s.as_str() {
                    "cls" => Some(Act::Clear),
                    "key" => Some(Act::GetKey),
                    "nl" => Some(Act::Nl),
                    "trc" => Some(Act::Trc),
                    "say" => Some(Act::Say(Unit::find_ref(vec!["msg".into()].into_iter(), glob)?)),
                    _ => None
                },
                Or::Second((s, msg)) =>
                    match s.as_str() {
                        "say" => Some(Act::Say(msg)),
                        _ => None
                    }
            }
        })
    }
}

impl FromUnit for Term {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut term = Term::default();

        let schm_acts = SchemaOr(
            SchemaSeq(SchemaUnit),
            SchemaUnit
        );

        let schm = SchemaOr(
            SchemaMapEntry(
                Unit::Str("term".into()),
                schm_acts.clone()
            ),
            SchemaMapEntry(
                Unit::Str("term.gfx".into()),
                schm_acts
            )
        );

        schm.find_loc(u).map(|or| {
            let acts = match or {
                Or::First(acts) => {
                    term.mode = Mode::Cli;
                    acts
                },
                Or::Second(acts) => {
                    term.mode = Mode::Gfx;
                    acts
                }
            };

            let lst = match acts {
                Or::First(lst) => lst,
                Or::Second(act) => vec![act],
            };

            let acts = lst.into_iter().filter_map(|act| Act::from_unit(u, &act));

            acts.for_each(|act| {
                match term.act.as_mut() {
                    Some(acts) => acts.push(act),
                    None => term.act = Some(vec![act]),
                }
            });
        });

        return Some(term)
    }
}

impl ServHlr for Term {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Terminal I/O service\nExample: {msg:hello task:io.term}".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&mut self, msg: Msg, serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(acts) = self.act.clone() {
            for act in acts {
                act.act(self, &msg, kern)?;
            }
        } else {
            if let Some(_msg) = Unit::find_ref(vec!["msg".into()].into_iter(), &msg.msg) {
                let act = Act::Say(_msg);
                act.act(self, &msg, kern)?;
            }
        }

        Ok(Some(msg))
    }
}
