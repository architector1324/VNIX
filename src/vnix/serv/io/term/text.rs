use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::string::String;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::TaskLoop;
use crate::vnix::core::unit::{Unit, FromUnit, DisplayShort, SchemaPair, SchemaUnit, Schema, SchemaStr, SchemaOr, SchemaMapEntry, SchemaMapSecondRequire, SchemaBool, SchemaInt, SchemaRef, Or, SchemaMapRequire, SchemaMap};
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

pub type InpAsync<'a> = ThreadAsync<'a, Result<String, CLIErr>>;

#[derive(Debug, Clone)]
pub struct Inp {
    pmt: String,
    prs: bool,
    sct: bool,
    out: Vec<String>,
    pub mode: ActMode
}

// impl Say {
//     fn say_unit(&self, msg: &Unit, term: &Term, kern: &mut Kern) -> Result<(), CLIErr> {
//         if let Some(shrt) = self.shrt {
//             term.print(format!("{}", DisplayShort(&msg, shrt)).as_str(), &self.act_mode, kern)?;
//         } else {
//             term.print(format!("{}", msg).as_str(), &self.act_mode, kern)?;
//         }

//         Ok(())
//     }

//     fn say(&mut self, orig: &Unit, term: &Term, kern: &mut Kern) -> Result<(), CLIErr> {
//         match self.msg.clone() {
//             Unit::Str(out) => term.print(out.replace("\\n", "\n").replace("\\r", "\r").trim_matches('`'), &self.act_mode, kern)?,
//             Unit::Ref(path) => {
//                 if let Some(_msg) = Unit::find_ref(path.into_iter(), orig) {
//                     self.msg = _msg;
//                     return self.say(orig, term, kern);
//                 }
//             },
//             _ => return self.say_unit(&self.msg, term, kern)
//         }
//         Ok(())
//     }
// }

impl FromUnit for Say {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaOr(
                SchemaPair(SchemaStr, SchemaRef),
                SchemaPair(SchemaStr, SchemaUnit)
            ),
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
                Or::First(or) => {
                    let (s, msg) = match or {
                        Or::First((s, path)) => (s, Unit::Ref(path)),
                        Or::Second((s, msg)) => (s, msg)
                    };
            
                    match s.as_str() {
                        "say" => (msg, None, false, SayMode::Norm, ActMode::Cli),
                        "say.gfx" => (msg, None, false, SayMode::Norm, ActMode::Gfx),
                        "say.fmt" => (msg, None, false, SayMode::Fmt, ActMode::Cli),
                        "say.fmt.gfx" => (msg, None, false, SayMode::Fmt, ActMode::Gfx),
                        _ => return None
                    }
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

// impl TermAct for Say {
//     fn act<'a>(mut self, orig: Rc<Msg>, msg: Unit, term: Rc<Term>, kern: &'a Mutex<Kern>) -> TermActAsync<'a> {
//         match self.msg.clone() {
//             Unit::Ref(path) => {
//                 if let Some(_msg) = Unit::find_ref(path.into_iter(), &msg) {
//                     self.msg = _msg;
//                     return self.act(orig, msg, term, kern);
//                 } else {
//                     return Box::new(move || {
//                         yield;
//                         Ok(msg)
//                     })
//                 }
//             },
//             Unit::Stream(_msg, (serv, _)) => Box::new(move || {
//                 let task = TaskLoop::Chain {
//                     msg: *_msg,
//                     chain: vec![serv]
//                 };

//                 let id = kern.lock().reg_task(&orig.ath, "io.term.say", task)?;

//                 loop {
//                     let res = kern.lock().get_task_result(id);

//                     if let Some(res) = res {
//                         if let Some(_msg) = res? {
//                             self.msg = if let Some(_msg) = _msg.msg.as_map_find("msg") {
//                                 _msg
//                             } else {
//                                 _msg.msg
//                             };

//                             self.say(&msg, &term, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;

//                             if self.nl {
//                                 term.print("\n", &self.act_mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
//                             }
//                         }
//                         break;
//                     }
//                     yield;
//                 }

//                 term.flush(&self.act_mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
//                 yield;

//                 Ok(msg)
//             }),
//             Unit::Lst(seq) =>
//                 match self.mode {
//                     SayMode::Norm => Box::new(move || {
//                         self.say(&msg, &term, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;

//                         if self.nl {
//                             term.print("\n", &self.act_mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
//                         }
//                         yield;

//                         term.flush(&self.act_mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
//                         yield;

//                         Ok(msg)
//                     }),
//                     SayMode::Fmt => Box::new(move || {
//                         for u in seq {
//                             self.msg = u;
//                             let mut act = Box::into_pin(self.clone().act(orig.clone(), msg.clone(), term.clone(), kern));

//                             loop {
//                                 if let GeneratorState::Complete(res) = Pin::new(&mut act).resume(()) {
//                                     res?;
//                                     break;
//                                 }
//                                 yield;
//                             }
//                         }
//                         Ok(msg)
//                     })
//                 },
//             _ => Box::new(move || {
//                 self.say(&msg, &term, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;

//                 if self.nl {
//                     term.print("\n", &self.act_mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
//                 }
//                 yield;

//                 term.flush(&self.act_mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
//                 yield;

//                 Ok(msg)
//             })
//         } 
//     }
// }


impl FromUnit for Inp {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaMapRequire(
                SchemaMapEntry(Unit::Str("pmt".into()), SchemaStr),
                SchemaMapSecondRequire(
                    SchemaMapEntry(Unit::Str("prs".into()), SchemaBool),
                    SchemaMap(
                        SchemaMapEntry(Unit::Str("sct".into()), SchemaBool),
                        SchemaMapEntry(Unit::Str("out".into()), SchemaRef),
                    )
                )
            ),
            SchemaMapRequire(
                SchemaMapEntry(Unit::Str("pmt.gfx".into()), SchemaStr),
                SchemaMapSecondRequire(
                    SchemaMapEntry(Unit::Str("prs".into()), SchemaBool),
                    SchemaMap(
                        SchemaMapEntry(Unit::Str("sct".into()), SchemaBool),
                        SchemaMapEntry(Unit::Str("out".into()), SchemaRef),
                    )
                )
            ),
        );

        schm.find_deep(glob, u).map(|or| {
            let (pmt, prs, sct, out, mode) = match or {
                Or::First((pmt, (prs, (sct, out)))) => (pmt, prs, sct, out, ActMode::Cli),
                Or::Second((pmt, (prs, (sct, out)))) => (pmt, prs, sct, out, ActMode::Gfx)
            };

            Inp {
                pmt,
                prs: prs.unwrap_or(false),
                sct: sct.unwrap_or(false),
                out: out.unwrap_or(vec!["msg".into()]),
                mode
            }
        })
    }
}

// impl TermAct for Inp {
//     fn act<'a>(self, _orig: Rc<Msg>, msg: Unit, term: Rc<Term>, kern: &'a Mutex<Kern>) -> TermActAsync<'a> {
//         let hlr = move || {
//             // check input lock
//             loop {
//                 let kern_grd = kern.lock();

//                 if kern_grd.term.inp_lck {
//                     drop(kern_grd);
//                     yield;
//                     continue;
//                 }
//                 break;
//             }
//             kern.lock().term.inp_lck = true;
//             yield;

//             // process
//             term.print(self.pmt.as_str(), &self.mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
//             yield;

//             let mut gen = Box::into_pin(term.input(self.mode, self.sct, kern).0);

//             let out;
//             loop {
//                 if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
//                     kern.lock().term.inp_lck = false;
//                     out = res.map_err(|e| KernErr::CLIErr(e))?;
//                     break;
//                 }
//                 yield;
//             }

//             if out.is_empty() {
//                 return Ok(msg);
//             }

//             let out = if self.prs {
//                 Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0
//             } else {
//                 Unit::Str(out)
//             };

//             if let Some(u) = Unit::merge_ref(self.out.into_iter(), out, msg.clone()) {
//                 return Ok(u);
//             }
//             Ok(msg)
//         };
//         Box::new(hlr)
//     }
// }