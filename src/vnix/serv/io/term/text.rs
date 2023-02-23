use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::sync::Arc;
use spin::Mutex;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::TaskLoop;
use crate::vnix::core::unit::{Unit, FromUnit, DisplayShort, SchemaPair, SchemaUnit, Schema, SchemaStr};
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
        let schm = SchemaPair(SchemaStr, SchemaUnit);
        
        schm.find_deep(glob, u).and_then(|(s, u)| {
            match s.as_str() {
                "say" => Some(Say {
                    msg: u,
                    shrt: None,
                    nl: false,
                    mode: SayMode::Norm,
                    act_mode: ActMode::Cli
                }),
                "say.gfx" => Some(Say {
                    msg: u,
                    shrt: None,
                    nl: false,
                    mode: SayMode::Norm,
                    act_mode: ActMode::Gfx
                }),
                "say.fmt" => Some(Say {
                    msg: u,
                    shrt: None,
                    nl: false,
                    mode: SayMode::Fmt,
                    act_mode: ActMode::Cli
                }),
                "say.fmt.gfx" => Some(Say {
                    msg: u,
                    shrt: None,
                    nl: false,
                    mode: SayMode::Fmt,
                    act_mode: ActMode::Gfx
                }),
                _ => None
            }
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
