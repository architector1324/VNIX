use alloc::{vec, format};
use alloc::string::String;
use alloc::vec::Vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapSecondRequire, SchemaMapEntry, SchemaBool, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMap, SchemaMapRequire, SchemaRef, SchemaPair, SchemaOr, Or, DisplayShort};

use super::{TermAct, Mode, Term};


#[derive(Debug, Clone)]
pub enum SayMode {
    Norm,
    Fmt,
}

#[derive(Debug, Clone)]
pub struct Say {
    pub msg: Unit,
    pub shrt: Option<usize>,
    pub nl: bool,
    pub mode: SayMode
}

#[derive(Debug, Clone)]
pub struct Inp {
    pub pmt: String,
    pub prs: bool,
    pub sct: bool,
    pub out: Vec<String>
}

#[derive(Debug, Clone)]
pub struct Put {
    pos: (i32, i32),
    str: String
}

impl Say {
    fn say(&self, msg: &Unit, term: &mut Term, kern: &mut Kern) -> Result<(), KernErr> {
        if let Some(shrt) = self.shrt {
            term.print(format!("{}", DisplayShort(&msg, shrt)).as_str(), kern)?;
        } else {
            term.print(format!("{}", msg).as_str(), kern)?;
        }
        Ok(())
    }
}

impl FromUnit for Say {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("shrt".into()), SchemaInt),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("nl".into()), SchemaBool),
                SchemaOr(
                    SchemaOr(
                        SchemaMapEntry(Unit::Str("say".into()), SchemaRef),
                        SchemaMapEntry(Unit::Str("say".into()), SchemaUnit),
                    ),
                    SchemaOr(
                        SchemaMapEntry(Unit::Str("say.fmt".into()), SchemaRef),
                        SchemaMapEntry(Unit::Str("say.fmt".into()), SchemaUnit),
                    ),
                )
            )
        );

        schm.find_deep(glob, u).and_then(|(shrt, (nl, or))| {
            let (or, mode) = match or {
                Or::First(or) => (or, SayMode::Norm),
                Or::Second(or) => (or, SayMode::Fmt)
            };

            let msg = match or {
                Or::First(path) => Unit::Ref(path),
                Or::Second(u) => u
            };

            Some(Say {
                msg,
                shrt: shrt.map(|shrt| shrt as usize),
                nl: nl.unwrap_or(false),
                mode
            })
        })
    }
}


impl FromUnit for Inp {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(Unit::Str("pmt".into()), SchemaStr),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("prs".into()), SchemaBool),
                SchemaMap(
                    SchemaMapEntry(Unit::Str("sct".into()), SchemaBool),
                    SchemaMapEntry(Unit::Str("out".into()), SchemaRef),

                )
            )
        );

        schm.find_deep(glob, u).map(|(pmt, (prs, (sct, out)))| {
            Inp {
                pmt,
                prs: prs.unwrap_or(false),
                sct: sct.unwrap_or(false),
                out: out.unwrap_or(vec!["msg".into()])
            }
        })
    }
}

impl FromUnit for Put {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(
                Unit::Str("pos".into()),
                SchemaPair(SchemaInt, SchemaInt)
            ),
            SchemaMapEntry(
                Unit::Str("put".into()),
                SchemaStr
            )
        );

        schm.find_deep(glob, u).map(|(pos, str)| {
            Put {pos, str}
        })
    }
}

impl TermAct for Say {
    fn act(mut self, term: &mut Term, orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        match self.msg.clone() {
            Unit::Str(s) => term.print(format!("{}", s.replace("\\n", "\n").replace("\\r", "\r")).as_str(), kern)?,
            Unit::Ref(path) => {
                if let Some(_msg) = Unit::find_ref(path.into_iter(), &msg) {
                    self.msg = _msg;
                    return self.act(term, orig, msg, kern);
                } else {
                    return Ok(msg)
                }
            },
            Unit::Stream(_msg, (serv, _addr)) => {
                let _msg = kern.msg(&orig.ath, *_msg)?;

                todo!()
                // if let Some(_msg) = kern.send(serv.as_str(), _msg)? {
                //     if let Some(_msg) = _msg.msg.as_map_find("msg") {
                //         self.msg = _msg;
                //         return self.act(term, orig, msg, kern);
                //     }
                // }
            },
            Unit::Lst(lst) => 
                match self.mode {
                    SayMode::Norm => self.say(&Unit::Lst(lst), term, kern)?,
                    SayMode::Fmt => {
                        for u in lst {
                            match u {
                                Unit::Str(s) => term.print(format!("{}", s.replace("\\n", "\n").replace("\\r", "\r")).as_str(), kern)?,
                                Unit::Ref(path) => {
                                    if let Some(_msg) = Unit::find_ref(path.into_iter(), &msg) {
                                        self.say(&_msg, term, kern)?
                                    }
                                },
                                _ => self.say(&u, term, kern)?
                            }
                        }
                    }
                },
            _ => self.say(&self.msg, term, kern)?
        }

        if self.nl {
            term.print(format!("\n").as_str(), kern)?;
        }

        Ok(msg)
    }
}


impl TermAct for Inp {
    fn act(self, term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        term.print(self.pmt.as_str(), kern)?;
        let out = term.input(self.sct, kern)?;

        if out.is_empty() {
            return Ok(msg);
        }

        let out = if self.prs {
            Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0
        } else {
            Unit::Str(out)
        };

        if let Some(u) = Unit::merge_ref(self.out.into_iter(), out, msg.clone()) {
            return Ok(u);
        }
        Ok(msg)
    }
}

impl TermAct for Put {
    fn act(self, term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        match term.mode {
            Mode::Cli => {
                let (w, h) = kern.drv.cli.res().map_err(|e| KernErr::CLIErr(e))?;

                if self.pos.0 < w as i32 && self.pos.1 < h as i32 {
                    if let Some(ch) = self.str.chars().next() {
                        term.print_glyth(ch, ((self.pos.0 * 8) as usize, (self.pos.1 * 16) as usize), 0x00ff00, kern)?;
                    }
                }
            },
            Mode::Gfx => {
                let (w, h) = kern.drv.disp.res().map_err(|e| KernErr::DispErr(e))?;
                let (w, h) = (w / 8, h / 16);

                if self.pos.0 < w as i32 && self.pos.1 < h as i32 {
                    if let Some(ch) = self.str.chars().next() {
                        term.print_glyth(ch, ((self.pos.0 * 8) as usize, (self.pos.1 * 16) as usize), 0x00ff00, kern)?;
                    }
                }
            }
        }

        Ok(msg)
    }
}
