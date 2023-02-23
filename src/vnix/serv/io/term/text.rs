use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::sync::Arc;
use spin::Mutex;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::TaskLoop;
use crate::vnix::core::unit::{Unit, FromUnit, DisplayShort, SchemaPair, SchemaUnit, Schema, SchemaStr, SchemaOr, SchemaMapEntry, SchemaMapSecondRequire, SchemaBool, SchemaInt, SchemaRef, Or};
use crate::vnix::core::kern::{Kern, KernErr};

use super::{TermActAsync, Term, TermAct, ActMode};


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
    pub mode: SayMode,
    pub act_mode: ActMode
}


impl Say {
    fn say_unit(&self, msg: &Unit, term: &Term, kern: &mut Kern) -> Result<(), CLIErr> {
        if let Some(shrt) = self.shrt {
            term.print(format!("{}", DisplayShort(&msg, shrt)).as_str(), &self.act_mode, kern)?;
        } else {
            term.print(format!("{}", msg).as_str(), &self.act_mode, kern)?;
        }

        Ok(())
    }

    fn say(&mut self, orig: &Unit, term: &Term, kern: &mut Kern) -> Result<(), CLIErr> {
        match self.msg.clone() {
            Unit::Str(out) => term.print(out.replace("\\n", "\n").replace("\\r", "\r").trim_matches('`'), &self.act_mode, kern)?,
            Unit::Ref(path) => {
                if let Some(_msg) = Unit::find_ref(path.into_iter(), orig) {
                    self.msg = _msg;
                    return self.say(orig, term, kern);
                }
            },
            _ => return self.say_unit(&self.msg, term, kern)
        }
        Ok(())
    }
}

impl FromUnit for Say {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaPair(SchemaStr, SchemaUnit),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("shrt".into()), SchemaInt),
                SchemaMapSecondRequire(
                    SchemaMapEntry(Unit::Str("nl".into()), SchemaBool),
                    SchemaOr(
                        SchemaOr(
                            SchemaOr(
                                SchemaMapEntry(Unit::Str("say".into()), SchemaRef),
                                SchemaMapEntry(Unit::Str("say".into()), SchemaUnit)
                            ),
                            SchemaOr(
                                SchemaMapEntry(Unit::Str("say.fmt".into()), SchemaRef),
                                SchemaMapEntry(Unit::Str("say.fmt".into()), SchemaUnit)
                            )
                        ),
                        SchemaOr(
                            SchemaOr(
                                SchemaMapEntry(Unit::Str("say.gfx".into()), SchemaRef),
                                SchemaMapEntry(Unit::Str("say.gfx".into()), SchemaUnit)
                            ),
                            SchemaOr(
                                SchemaMapEntry(Unit::Str("say.fmt.gfx".into()), SchemaRef),
                                SchemaMapEntry(Unit::Str("say.fmt.gfx".into()), SchemaUnit)
                            )
                        )
                    )
                )
            )
        );

        schm.find_deep(glob, u).and_then(|or| {
            let (msg, shrt, nl, mode, act_mode) = match or {
                Or::First((s, msg)) =>
                    match s.as_str() {
                        "say" => (msg, None, false, SayMode::Norm, ActMode::Cli),
                        "say.gfx" => (msg, None, false, SayMode::Norm, ActMode::Gfx),
                        "say.fmt" => (msg, None, false, SayMode::Fmt, ActMode::Cli),
                        "say.fmt.gfx" => (msg, None, false, SayMode::Fmt, ActMode::Gfx),
                        _ => return None
                    },
                Or::Second((shrt,(nl, or))) =>
                    match or {
                        Or::First(say_cli) =>
                            match say_cli {
                                Or::First(say) =>
                                    match say {
                                        Or::First(path) => (Unit::Ref(path), shrt, nl.unwrap_or(false), SayMode::Norm, ActMode::Cli),
                                        Or::Second(msg) => (msg, shrt, nl.unwrap_or(false), SayMode::Norm, ActMode::Cli)
                                    },
                                Or::Second(say_fmt) =>
                                    match say_fmt {
                                        Or::First(path) => (Unit::Ref(path), shrt, nl.unwrap_or(false), SayMode::Fmt, ActMode::Cli),
                                        Or::Second(msg) => (msg, shrt, nl.unwrap_or(false), SayMode::Fmt, ActMode::Cli)
                                    }
                            },
                        Or::Second(say_gfx) =>
                            match say_gfx {
                                Or::First(say) =>
                                    match say {
                                        Or::First(path) => (Unit::Ref(path), shrt, nl.unwrap_or(false), SayMode::Norm, ActMode::Gfx),
                                        Or::Second(msg) => (msg, shrt, nl.unwrap_or(false), SayMode::Norm, ActMode::Gfx)
                                    },
                                Or::Second(say_fmt) =>
                                    match say_fmt {
                                        Or::First(path) => (Unit::Ref(path), shrt, nl.unwrap_or(false), SayMode::Fmt, ActMode::Gfx),
                                        Or::Second(msg) => (msg, shrt, nl.unwrap_or(false), SayMode::Fmt, ActMode::Gfx)
                                    }
                            },
                    }
            };

            Some(Say {
                msg,
                shrt: shrt.map(|v| v as usize),
                nl,
                mode,
                act_mode
            })
        })
    }
}

impl TermAct for Say {
    fn act<'a>(mut self, orig: Arc<Msg>, msg: Unit, term: Arc<Term>, kern: &'a Mutex<Kern>) -> TermActAsync<'a> {
        match self.msg.clone() {
            Unit::Ref(path) => {
                if let Some(_msg) = Unit::find_ref(path.into_iter(), &msg) {
                    self.msg = _msg;
                    return self.act(orig, msg, term, kern);
                } else {
                    return TermActAsync(Box::new(move || {
                        yield;
                        Ok(msg)
                    }))
                }
            },
            Unit::Stream(_msg, (serv, _)) => TermActAsync(Box::new(move || {
                let task = TaskLoop::Chain {
                    msg: *_msg,
                    chain: vec![serv]
                };

                let id = kern.lock().reg_task(&orig.ath, "io.term.say", task)?;

                loop {
                    let res = kern.lock().get_task_result(id);

                    if let Some(res) = res {
                        if let Some(_msg) = res? {
                            self.msg = if let Some(_msg) = _msg.msg.as_map_find("msg") {
                                _msg
                            } else {
                                _msg.msg
                            };

                            self.say(&msg, &term, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;

                            if self.nl {
                                term.print("\n", &self.act_mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                            }
                        }
                        break;
                    }
                    yield;
                }

                Ok(msg)
            })),
            Unit::Lst(seq) =>
                match self.mode {
                    SayMode::Norm => TermActAsync(Box::new(move || {
                        self.say(&msg, &term, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;

                        if self.nl {
                            term.print("\n", &self.act_mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                        }
                        yield;

                        Ok(msg)
                    })),
                    SayMode::Fmt => TermActAsync(Box::new(move || {
                        for u in seq {
                            self.msg = u;
                            let mut act = Box::into_pin(self.clone().act(orig.clone(), msg.clone(), term.clone(), kern).0);

                            loop {
                                if let GeneratorState::Complete(res) = Pin::new(&mut act).resume(()) {
                                    res?;
                                    break;
                                }
                                yield;
                            }
                        }
                        Ok(msg)
                    }))
                },
            _ => TermActAsync(Box::new(move || {
                self.say(&msg, &term, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;

                if self.nl {
                    term.print("\n", &self.act_mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                }
                yield;

                Ok(msg)
            }))
        } 
    }
}
