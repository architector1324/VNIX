mod text;
mod content;

use core::pin::Pin;
use core::fmt::Write;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::sync::Arc;

use alloc::{vec, format};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};

use crate::driver::{CLIErr, DispErr, TermKey};
use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::TaskLoop;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaStr, Schema, SchemaMapEntry, SchemaUnit, SchemaOr, SchemaSeq, Or, SchemaPair, SchemaRef, SchemaInt};
use crate::vnix::core::kern::{Kern, KernErr, Addr};
use crate::vnix::core::serv::{ServHlrAsync, Serv, ServHlr, ServHelpTopic};


#[derive(Debug, Clone)]
pub enum ActMode {
    Cli,
    Gfx,
}

#[derive(Debug, Clone)]
enum GetResKind {
    Curr,
    All
}

#[derive(Debug, Clone)]
struct GetRes {
    kind: GetResKind,
    path: Vec<String>,
    mode: ActMode
}

#[derive(Debug, Clone)]
struct SetRes {
    size: (usize, usize),
    mode: ActMode
}

#[derive(Debug, Clone)]
struct GetKey(Option<Vec<String>>);

#[derive(Debug)]
pub struct TermBase {
    pos: (usize, usize),
    inp_lck: bool
}

#[derive(Debug)]
pub struct Font {
    glyths: Vec<(char, [u8; 16])>
}

#[derive(Debug)]
pub struct TermRes {
    pub font: Font,
}

pub struct TermActAsync<'a>(Box<dyn Generator<Yield = (), Return = Result<Unit, KernErr>> + 'a>);

pub trait TermAct {
    fn act<'a>(self, orig: Arc<Msg>, msg: Unit, term: Arc<Term>, kern: &'a Mutex<Kern>) -> TermActAsync<'a>;
}

#[derive(Debug, Clone)]
enum ActKind {
    Cls,
    Nl,
    Trc,
    GetRes(GetRes),
    SetRes(SetRes),
    GetKey(GetKey),
    Stream(Unit, (String, Addr)),
    Say(text::Say),
    Inp(text::Inp)
}

#[derive(Debug, Clone)]
struct Act {
    kind: ActKind,
    mode: ActMode
}

#[derive(Debug)]
pub struct Term {
    acts: Option<Vec<Act>>,
    res: TermRes
}

impl Term {
    fn clear(&self, mode: &ActMode, kern: &mut Kern) -> Result<(), CLIErr> {
        match mode {
            ActMode::Cli => kern.drv.cli.clear()?,
            ActMode::Gfx => kern.drv.disp.fill(&|_, _| 0x000000).map_err(|_| CLIErr::Clear)?
        }
        kern.term.pos = (0, 0);

        Ok(())
    }

    fn clear_line(&self, mode: &ActMode, kern: &mut Kern) -> Result<(), CLIErr> {
        match mode {
            ActMode::Cli => write!(kern.drv.cli, "\r").map_err(|_| CLIErr::Clear)?,
            ActMode::Gfx => {
                let (w, _) = kern.drv.disp.res().map_err(|_| CLIErr::Clear)?;

                kern.term.pos.0 = 0;

                for _ in 0..(w / 8 - 1) {
                    self.print(" ", mode, kern)?;
                }
                kern.term.pos.0 = 0;
            }
        }
        Ok(())
    }

    fn print_glyth(&self, ch: char, pos: (usize, usize), src: u32, mode: &ActMode, kern: &mut Kern) -> Result<(), CLIErr> {
        match mode {
            ActMode::Cli => {
                kern.drv.cli.glyth(ch, (pos.0 / 8, pos.1 / 16))?;
            },
            ActMode::Gfx => {
                let img = self.res.font.glyths.iter().find(|(_ch, _)| *_ch == ch).map_or(Err(CLIErr::Write), |(_, img)| Ok(img))?;

                let mut tmp = Vec::with_capacity(8 * 16);

                for y in 0..16 {
                    for x in 0..8 {
                        let px = if (img[y] >> (8 - x)) & 1 == 1 {0xffffff} else {0x000000};
                        tmp.push(px);
                    }
                }
                kern.drv.disp.blk((pos.0 as i32, pos.1 as i32), (8, 16), src, tmp.as_slice()).map_err(|_| CLIErr::Write)?;
            }
        }
        Ok(())
    }

    fn print(&self, out: &str, mode: &ActMode, kern: &mut Kern) ->  Result<(), CLIErr> {
        match mode {
            ActMode::Cli => {
                let (w, _) = kern.drv.cli.res()?;

                for ch in out.chars() {
                    if ch == '\n' {
                        kern.term.pos.1 += 1;
                        kern.term.pos.0 = 0;
                    } else if ch == '\r' {
                        self.clear_line(mode, kern)?;
                    } else if ch == '\u{8}' {
                        if kern.term.pos.0 == 0 && kern.term.pos.1 > 0 {
                            kern.term.pos.1 -= 1;
                        } else {
                            kern.term.pos.0 -= 1;
                        }
                    } else {
                        kern.term.pos.0 += 1;
                    }

                    if kern.term.pos.0 >= w {
                        kern.term.pos.1 += 1;
                        kern.term.pos.0 = 0;
                    }

                    write!(kern.drv.cli, "{}", ch).map_err(|_| CLIErr::Write)?;
                }
            },
            ActMode::Gfx => {
                let (w, _) = kern.drv.disp.res().map_err(|_| CLIErr::Write)?;

                for ch in out.chars() {
                    if ch == '\n' {
                        kern.term.pos.1 += 1;
                        kern.term.pos.0 = 0;
                    } else if ch == '\r' {
                        self.clear_line(mode, kern)?;
                    } else if ch == '\u{8}' {
                        if kern.term.pos.0 == 0 && kern.term.pos.1 > 0 {
                            kern.term.pos.1 -= 1;
                        } else {
                            kern.term.pos.0 -= 1;
                        }
                        self.print_glyth(' ', (kern.term.pos.0 * 8, kern.term.pos.1 * 16), 0x00ff00, mode, kern)?;
                    } else {
                        self.print_glyth(ch, (kern.term.pos.0 * 8, kern.term.pos.1 * 16), 0x00ff00, mode, kern)?;
                        kern.term.pos.0 += 1;
                    }

                    if kern.term.pos.0 * 8 >= w {
                        kern.term.pos.1 += 1;
                        kern.term.pos.0 = 0;
                    }
                }
            }
        }
        Ok(())
    }

    fn get_key(&self, kern: &mut Kern) -> Result<Option<TermKey>, CLIErr> {
        kern.drv.cli.get_key(false)
    }

    fn input<'a>(self: Arc<Self>, mode: ActMode, secret: bool, kern: &'a Mutex<Kern>) -> text::InpAsync {
        let hlr = move || {
            let mut out = String::new();
            let save_cur = kern.lock().term.pos.clone();

            self.flush(&mode, &mut kern.lock()).map_err(|_| CLIErr::Write)?;
            yield;

            // process
            loop {
                let mut kern_grd = kern.lock();

                if let Some(key) = kern_grd.drv.cli.get_key(false)? {
                    if let TermKey::Char(c) = key {
                        if c == '\r' || c == '\n' {
                            break;
                        } else if c == '\u{8}' && kern_grd.term.pos.0 > save_cur.0 {
                            out.pop();
                            self.print(format!("{}", c).as_str(), &mode, &mut kern_grd)?;
                            self.flush(&mode, &mut kern_grd).map_err(|_| CLIErr::Write)?;
                        } else if !c.is_ascii_control() {
                            write!(out, "{}", c).map_err(|_| CLIErr::Write)?;
                            if !secret {
                                self.print(format!("{}", c).as_str(), &mode, &mut kern_grd)?;
                                self.flush(&mode, &mut kern_grd).map_err(|_| CLIErr::Write)?;
                            }
                        }
                    }
                }

                drop(kern_grd);
                yield;
            }
            Ok(out)
        };
        text::InpAsync(Box::new(hlr))
    }

    fn flush(&self, mode: &ActMode, kern: &mut Kern) -> Result<(), DispErr> {
        if let ActMode::Gfx = mode {
            kern.drv.disp.flush()?;
        }
        Ok(())
    }
}

impl Default for TermBase {
    fn default() -> Self {
        TermBase {
            pos: (0, 0),
            inp_lck: false
        }
    }
}

impl Default for Term {
    fn default() -> Self {
        Term {
            acts: None,
            res: TermRes {
                font: Font {
                    glyths: content::SYS_FONT.to_vec()
                }
            }
        }
    }
}

impl FromUnit for GetRes {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaPair(SchemaStr, SchemaRef);

        schm.find_deep(glob, u).and_then(|(s, path)| {
            match s.as_str() {
                "get.res" => Some(GetRes {
                    kind: GetResKind::Curr,
                    path,
                    mode: ActMode::Cli
                }),
                "get.res.gfx" => Some(GetRes {
                    kind: GetResKind::Curr,
                    path,
                    mode: ActMode::Gfx
                }),
                "get.res.lst" => Some(GetRes {
                    kind: GetResKind::All,
                    path,
                    mode: ActMode::Cli
                }),
                "get.res.lst.gfx" => Some(GetRes {
                    kind: GetResKind::All,
                    path,
                    mode: ActMode::Gfx
                }),
                _ => None
            }
        })
    }
}

impl FromUnit for SetRes {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaPair(
            SchemaStr,
            SchemaPair(SchemaInt, SchemaInt)
        );

        schm.find_deep(glob, u).and_then(|(s, (w, h))| {
            match s.as_str() {
                "set.res" => Some(SetRes {
                    size: (w as usize, h as usize),
                    mode: ActMode::Cli
                }),
                "set.res.gfx" => Some(SetRes {
                    size: (w as usize, h as usize),
                    mode: ActMode::Gfx
                }),
                _ => None
            }
        })
    }
}

impl FromUnit for GetKey {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaPair(SchemaStr, SchemaRef);

        schm.find_deep(glob, u).and_then(|(s, path)| {
            match s.as_str() {
                "key" => Some(GetKey(Some(path))),
                _ => None
            }
        })
    }
}

impl FromUnit for Act {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaStr,
            SchemaUnit
        );

        schm.find_deep(glob, u).and_then(|or| {
            match or {
                Or::First(s) =>
                    match s.as_str() {
                        "cls" => Some(Act {
                            kind: ActKind::Cls,
                            mode: ActMode::Cli
                        }),
                        "cls.gfx" => Some(Act {
                            kind: ActKind::Cls,
                            mode: ActMode::Gfx
                        }),
                        "nl" => Some(Act {
                            kind: ActKind::Nl,
                            mode: ActMode::Cli
                        }),
                        "nl.gfx" => Some(Act {
                            kind: ActKind::Nl,
                            mode: ActMode::Gfx
                        }),
                        "trc" => Some(Act {
                            kind: ActKind::Trc,
                            mode: ActMode::Cli
                        }),
                        "trc.gfx" => Some(Act {
                            kind: ActKind::Trc,
                            mode: ActMode::Gfx
                        }),
                        "get.res" => Some(Act {
                            kind: ActKind::GetRes(GetRes {
                                kind: GetResKind::Curr,
                                path: vec!["msg".into()],
                                mode: ActMode::Cli
                            }),
                            mode: ActMode::Cli
                        }),
                        "get.res.gfx" => Some(Act {
                            kind: ActKind::GetRes(GetRes {
                                kind: GetResKind::Curr,
                                path: vec!["msg".into()],
                                mode: ActMode::Gfx
                            }),
                            mode: ActMode::Gfx
                        }),
                        "get.res.lst" => Some(Act {
                            kind: ActKind::GetRes(GetRes {
                                kind: GetResKind::All,
                                path: vec!["msg".into()],
                                mode: ActMode::Cli
                            }),
                            mode: ActMode::Cli
                        }),
                        "get.res.lst.gfx" => Some(Act {
                            kind: ActKind::GetRes(GetRes {
                                kind: GetResKind::All,
                                path: vec!["msg".into()],
                                mode: ActMode::Gfx
                            }),
                            mode: ActMode::Gfx
                        }),
                        "key" => Some(Act {
                            kind: ActKind::GetKey(GetKey(None)),
                            mode: ActMode::Cli
                        }),
                        "say" => Some(Act {
                            kind: ActKind::Say(text::Say {
                                msg: Unit::Ref(vec!["msg".into()]),
                                shrt: None,
                                nl: false,
                                mode: text::SayMode::Norm,
                                act_mode: ActMode::Cli
                            }),
                            mode: ActMode::Cli
                        }),
                        "say.gfx" => Some(Act {
                            kind: ActKind::Say(text::Say {
                                msg: Unit::Ref(vec!["msg".into()]),
                                shrt: None,
                                nl: false,
                                mode: text::SayMode::Norm,
                                act_mode: ActMode::Gfx
                            }),
                            mode: ActMode::Cli
                        }),
                        "say.fmt" => Some(Act {
                            kind: ActKind::Say(text::Say {
                                msg: Unit::Ref(vec!["msg".into()]),
                                shrt: None,
                                nl: false,
                                mode: text::SayMode::Fmt,
                                act_mode: ActMode::Cli
                            }),
                            mode: ActMode::Cli
                        }),
                        "say.fmt.gfx" => Some(Act {
                            kind: ActKind::Say(text::Say {
                                msg: Unit::Ref(vec!["msg".into()]),
                                shrt: None,
                                nl: false,
                                mode: text::SayMode::Fmt,
                                act_mode: ActMode::Gfx
                            }),
                            mode: ActMode::Cli
                        }),
                        _ => None
                    },
                Or::Second(u) => {
                    if let Unit::Stream(msg, (serv, addr)) = u {
                        return Some(Act {
                            mode: ActMode::Cli,
                            kind: ActKind::Stream(*msg, (serv, addr))
                        });
                    }

                    if let Some(get_res) = GetRes::from_unit(glob, &u) {
                        return Some(Act {
                            mode: get_res.mode.clone(),
                            kind: ActKind::GetRes(get_res)
                        })
                    }

                    if let Some(set_res) = SetRes::from_unit(glob, &u) {
                        return Some(Act {
                            mode: set_res.mode.clone(),
                            kind: ActKind::SetRes(set_res)
                        })
                    }

                    if let Some(get_key) = GetKey::from_unit(glob, &u) {
                        return Some(Act {
                            kind: ActKind::GetKey(get_key),
                            mode: ActMode::Cli
                        })
                    }

                    if let Some(say) = text::Say::from_unit(glob, &u) {
                        return Some(Act {
                            mode: say.act_mode.clone(),
                            kind: ActKind::Say(say)
                        });
                    }

                    if let Some(inp) = text::Inp::from_unit(glob, &u) {
                        return Some(Act {
                            mode: inp.mode.clone(),
                            kind: ActKind::Inp(inp)
                        })
                    }

                    None
                }
            }
        })
    }
}

impl FromUnit for Term {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut term = Term::default();

        let schm = SchemaOr(
            SchemaSeq(SchemaUnit),
            SchemaOr(
                SchemaMapEntry(
                    Unit::Str("term".into()),
                    SchemaOr(
                        SchemaSeq(SchemaUnit),
                        SchemaUnit
                    )
                ),
                SchemaUnit
            )
        );

        term.acts = schm.find_loc(u).and_then(|or| {
            let lst = match or {
                Or::First(seq) => seq,
                Or::Second(or) =>
                    match or {
                        Or::First(or) =>
                            match or {
                                Or::First(seq) => seq,
                                Or::Second(act) => vec![act]
                            }
                        Or::Second(act) => vec![act]
                    }
            };

            let acts = lst.into_iter().map(|act| Act::from_unit(&u, &act)).collect::<Option<Vec<_>>>()?;
            Some(acts)
        });

        Some(term)
    }
}

impl TermAct for GetRes {
    fn act<'a>(self, _orig: Arc<Msg>, msg: Unit, _term: Arc<Term>, kern: &'a Mutex<Kern>) -> TermActAsync<'a> {
        let hlr = move || {
            let _msg = match self.mode {
                ActMode::Cli =>
                    match self.kind {
                        GetResKind::Curr => {
                            let res = kern.lock().drv.cli.res().map_err(|e| KernErr::CLIErr(e))?;
                            Unit::Pair(
                                Box::new(Unit::Int(res.0 as i32)),
                                Box::new(Unit::Int(res.1 as i32))
                            )
                        },
                        GetResKind::All => {
                            let res = kern.lock().drv.cli.res_list().map_err(|e| KernErr::CLIErr(e))?;
                            Unit::Lst(res.into_iter().map(|(w, h)| {
                                Unit::Pair(
                                    Box::new(Unit::Int(w as i32)),
                                    Box::new(Unit::Int(h as i32))
                                )
                            }).collect())
                        }
                    },
                ActMode::Gfx =>
                    match self.kind {
                        GetResKind::Curr => {
                            let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DispErr(e))?;
                            Unit::Pair(
                                Box::new(Unit::Int(res.0 as i32)),
                                Box::new(Unit::Int(res.1 as i32))
                            )
                        },
                        GetResKind::All => {
                            let res = kern.lock().drv.disp.res_list().map_err(|e| KernErr::DispErr(e))?;
                            Unit::Lst(res.into_iter().map(|(w, h)| {
                                Unit::Pair(
                                    Box::new(Unit::Int(w as i32)),
                                    Box::new(Unit::Int(h as i32))
                                )
                            }).collect())
                        }
                    }
            };
            yield;

            if let Some(_msg) = Unit::merge_ref(self.path.into_iter(), _msg, msg.clone()) {
                return Ok(_msg);
            }
            Ok(msg)
        };
        TermActAsync(Box::new(hlr))
    }
}

impl TermAct for Act {
    fn act<'a>(self, orig: Arc<Msg>, mut msg: Unit, term: Arc<Term>, kern: &'a Mutex<Kern>) -> TermActAsync<'a> {
        match self.kind {
            ActKind::Cls => TermActAsync(Box::new(move || {
                term.clear(&self.mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                term.flush(&self.mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
                yield;

                Ok(msg)
            })),
            ActKind::Nl => TermActAsync(Box::new(move || {
                term.print("\n", &self.mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                term.flush(&self.mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
                yield;

                Ok(msg)
            })),
            ActKind::Trc => TermActAsync(Box::new(move || {
                term.print(orig.to_string().as_str(), &self.mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                term.flush(&self.mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
                yield;

                Ok(msg)
            })),
            ActKind::GetRes(get_res) => get_res.act(orig, msg, term, kern),
            ActKind::SetRes(set_res) => TermActAsync(Box::new(move || {
                match set_res.mode {
                    ActMode::Cli => kern.lock().drv.cli.set_res(set_res.size).map_err(|e| KernErr::CLIErr(e))?,
                    ActMode::Gfx => kern.lock().drv.disp.set_res(set_res.size).map_err(|e| KernErr::DispErr(e))?
                }
                yield;

                Ok(msg)
            })),
            ActKind::GetKey(get_key) => TermActAsync(Box::new(move || {
                term.flush(&self.mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
                yield;

                loop {
                    if let Some(key) = term.get_key(& mut kern.lock()).map_err(|e| KernErr::CLIErr(e))? {
                        if let Some(path) = get_key.0 {
                            if let Some(_msg) = Unit::merge_ref(path.into_iter(), Unit::Str(format!("{}", key)), msg.clone()) {
                                return Ok(_msg);
                            }
                        }
                        break;
                    }
                    yield;
                }

                Ok(msg)
            })),
            ActKind::Stream(_msg, (serv, _)) => TermActAsync(Box::new(move || {
                // run stream
                let task = TaskLoop::Chain {
                    msg: _msg,
                    chain: vec![serv]
                };

                let id = kern.lock().reg_task(&orig.ath, "io.term", task)?;
                let mut act = None;

                loop {
                    let res = kern.lock().get_task_result(id);

                    if let Some(res) = res {
                        if let Some(_msg) = res? {
                            let act_u = if let Some(_msg) = _msg.msg.as_map_find("msg") {
                                _msg
                            } else {
                                _msg.msg
                            };

                            act = Act::from_unit(&orig.msg, &act_u);
                        }
                        break;
                    }

                    yield;
                }

                // run action
                if let Some(act) = act {
                    let mut gen = Box::into_pin(act.act(orig, msg.clone(), term, kern).0);

                    loop {
                        if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
                            msg = msg.merge(res?);
                            break;
                        }
                        yield;
                    }
                    yield;

                    return Ok(msg);
                }

                Ok(msg)
            })),
            ActKind::Say(say) => say.act(orig, msg, term, kern),
            ActKind::Inp(inp) => inp.act(orig, msg, term, kern)
        }
    }
}

impl ServHlr for Term {
    fn help<'a>(self, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Terminal I/O service\nExample: hello@io.term\nFor gfx mode: (say.gfx hello)@io.term".into())
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
            // writeln!(kern.lock().drv.cli, "io.term: {:?}", self.acts);

            if let Some(acts) = self.acts.clone() {
                let mut out_u = msg.msg.clone();

                let term = Arc::new(self);
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