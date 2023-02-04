mod content;
mod ui;

use core::fmt::Write;

use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::string::String;
use alloc::vec::Vec;

use crate::driver::{CLIErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaStr, SchemaUnit, Schema, SchemaPair, SchemaSeq, Or, SchemaRef, SchemaOr, SchemaMapSeq, SchemaByte};

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};


pub trait TermAct {
    fn act(self, term: &mut Term, orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr>;
}

#[derive(Debug, Clone)]
enum GetRes {
    Cli,
    Gfx,
    ListCli,
    ListGfx
}

#[derive(Debug, Clone)]
enum SetRes {
    Cli,
    Gfx
}

#[derive(Debug, Clone)]
enum Act {
    Clear,
    Nl,
    GetKey(Option<Vec<String>>),
    Trc,
    GetRes(GetRes, Vec<String>),
    SetRes(SetRes, (usize, usize)),
    Say(ui::Say),
    Inp(ui::Inp),
    Put(ui::Put),
    Img(ui::Img),
    Spr(ui::Sprite),
    Vid(ui::Video),
    Win(ui::Win)
}

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
    font: Font,
    act: Option<Vec<Act>>
}

#[derive(Debug)]
struct Font {
    glyths: Vec<(char, [u8; 16])>
}

impl TermAct for Act {
    fn act(self, term: &mut Term, orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        match self {
            Act::Clear => term.clear(kern)?,
            Act::Nl => term.print("\n", kern)?,
            Act::Trc => term.print(format!("{}", orig).as_str(), kern)?,
            Act::Say(say) => return say.act(term, orig, msg, kern),
            Act::GetKey(may_path) => {
                term.flush(kern)?;

                if let Some(key) = term.get_key(true, kern)? {
                    if let Some(path) = may_path {
                        if let Some(u) = Unit::merge_ref(path.into_iter(), Unit::Str(format!("{}", key)), msg.clone()) {
                            return Ok(u);
                        }
                    }
                }
            },
            Act::GetRes(which, path) => {
                let u = match which {
                    GetRes::Cli => {
                        let res = kern.cli.res().map_err(|e| KernErr::CLIErr(e))?;
                        Unit::Pair(
                            Box::new(Unit::Int(res.0 as i32)),
                            Box::new(Unit::Int(res.1 as i32))
                        )
                    },
                    GetRes::Gfx => {
                        let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                        Unit::Pair(
                            Box::new(Unit::Int(res.0 as i32)),
                            Box::new(Unit::Int(res.1 as i32))
                        )
                    },
                    GetRes::ListCli => {
                        let lst = kern.cli.res_list().map_err(|e| KernErr::CLIErr(e))?;
                        
                        Unit::Lst(
                            lst.into_iter().map(|(w, h)| {
                                Unit::Pair(
                                    Box::new(Unit::Int(w as i32)),
                                    Box::new(Unit::Int(h as i32))
                                )
                            }).collect()
                        )
                    },
                    GetRes::ListGfx => {
                        let lst = kern.disp.res_list().map_err(|e| KernErr::DispErr(e))?;
                        
                        Unit::Lst(
                            lst.into_iter().map(|(w, h)| {
                                Unit::Pair(
                                    Box::new(Unit::Int(w as i32)),
                                    Box::new(Unit::Int(h as i32))
                                )
                            }).collect()
                        )
                    }
                };

                if let Some(u) = Unit::merge_ref(path.into_iter(), u, msg.clone()) {
                    return Ok(u);
                }
            },
            Act::SetRes(which, res) =>
                match which {
                    SetRes::Cli => kern.cli.set_res(res).map_err(|e| KernErr::CLIErr(e))?,
                    SetRes::Gfx => kern.disp.set_res(res).map_err(|e| KernErr::DispErr(e))?
                }
            Act::Inp(inp) => return inp.act(term, orig, msg, kern),
            Act::Put(put) => return put.act(term, orig, msg, kern),
            Act::Img(img) => return img.act(term, orig, msg, kern),
            Act::Spr(spr) => return spr.act(term, orig, msg, kern),
            Act::Vid(vid) => return vid.act(term, orig, msg, kern),
            Act::Win(win) => return win.act(term, orig, msg, kern)
        }
        Ok(msg)
    }
}

impl Term {
    fn print_glyth(&mut self, ch: char, pos: (usize, usize), src: u32, kern: &mut Kern) -> Result<(), KernErr> {
        match self.mode {
            Mode::Cli => {
                kern.cli.glyth(ch, (pos.0 / 8, pos.1 / 16)).map_err(|e| KernErr::CLIErr(e))?;
            },
            Mode::Gfx => {
                let img = self.font.glyths.iter().find(|(_ch, _)| *_ch == ch).map_or(Err(KernErr::CLIErr(CLIErr::Write)), |(_, img)| Ok(img))?;

                let mut tmp = Vec::with_capacity(8 * 16);

                for y in 0..16 {
                    for x in 0..8 {
                        let px = if (img[y] >> (8 - x)) & 1 == 1 {0xffffff} else {0x000000};
                        tmp.push(px);
                    }
                }
                kern.disp.blk((pos.0 as i32, pos.1 as i32), (8, 16), src, tmp.as_slice()).map_err(|e| KernErr::DispErr(e))?;
            }
        }
        Ok(())
    }

    fn print(&mut self, s: &str, kern: &mut Kern) -> Result<(), KernErr> {
        match self.mode {
            Mode::Cli => {
                let (w, _) = kern.cli.res().map_err(|e| KernErr::CLIErr(e))?;

                for ch in s.chars() {
                    if ch == '\n' {
                        kern.term.pos.1 += 1;
                        kern.term.pos.0 = 0;
                    } else if ch == '\r' {
                        self.clear_line(kern)?;
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

                    write!(kern.cli, "{}", ch).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
                Ok(())
            },
            Mode::Gfx => {
                let (w, _) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                for ch in s.chars() {
                    if ch == '\n' {
                        kern.term.pos.1 += 1;
                        kern.term.pos.0 = 0;
                    } else if ch == '\r' {
                        self.clear_line(kern)?;
                    } else if ch == '\u{8}' {
                        if kern.term.pos.0 == 0 && kern.term.pos.1 > 0 {
                            kern.term.pos.1 -= 1;
                        } else {
                            kern.term.pos.0 -= 1;
                        }
                        self.print_glyth(' ', (kern.term.pos.0 * 8, kern.term.pos.1 * 16), 0x00ff00, kern)?;
                    } else {
                        self.print_glyth(ch, (kern.term.pos.0 * 8, kern.term.pos.1 * 16), 0x00ff00, kern)?;
                        kern.term.pos.0 += 1;
                    }

                    if kern.term.pos.0 * 8 >= w {
                        kern.term.pos.1 += 1;
                        kern.term.pos.0 = 0;
                    }
                }
                Ok(())
            }
        }
    }

    fn flush(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        if let Mode::Gfx = self.mode {
            kern.disp.flush().map_err(|e| KernErr::DispErr(e))?;
        }
        Ok(())
    }

    fn input(&mut self, secret: bool, kern: &mut Kern) -> Result<String, KernErr> {
        let mut out = String::new();

        let save_cur = kern.term.pos.clone();

        self.flush(kern)?;

        loop {
            if let Some(key) = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                if let TermKey::Char(c) = key {
                    if c == '\r' || c == '\n' {
                        break;
                    } else if c == '\u{8}' && kern.term.pos.0 > save_cur.0 {
                        out.pop();
                        self.print(format!("{}", c).as_str(), kern)?;
                        self.flush(kern)?;
                    } else if !c.is_ascii_control() {
                        write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        if !secret {
                            self.print(format!("{}", c).as_str(), kern)?;
                            self.flush(kern)?;
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    fn clear_line(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        match self.mode {
            Mode::Cli => write!(kern.cli, "\r").map_err(|_| KernErr::CLIErr(CLIErr::Clear)),
            Mode::Gfx => {
                let (w, _) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                kern.term.pos.0 = 0;
        
                for _ in 0..(w / 8 - 1) {
                    self.print(" ", kern)?;
                }
                kern.term.pos.0 = 0;

                Ok(())
            }
        }
    }

    fn clear(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        match self.mode {
            Mode::Cli => kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear)),
            Mode::Gfx => {
                kern.term.pos = (0, 0);
                kern.disp.fill(&|_, _| 0x000000).map_err(|e| KernErr::DispErr(e))
            }
        }
    }

    fn get_key(&mut self, block: bool, kern: &mut Kern) -> Result<Option<TermKey>, KernErr> {
        let key = kern.cli.get_key(block).map_err(|e| KernErr::CLIErr(e))?;
        Ok(key)
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
            font: Font{glyths: Vec::from(content::SYS_FONT)},
            act: None
        }
    }
}

impl FromUnit for Font {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapEntry(
            Unit::Str("font".into()),
            SchemaMapSeq(
                SchemaStr,
                SchemaSeq(SchemaByte)
            )
        );

        schm.find_deep(glob, u).map(|glyths| {
            let glyths = glyths.iter().filter_map(|(s, v)| {
                let dat = v.iter().cloned().map(|v| v as u8).collect::<Vec<_>>().try_into().ok()?;
                Some((s.chars().next()?, dat))}
            ).collect();

            Font {
                glyths
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
            SchemaOr(
                SchemaOr(
                    SchemaPair(SchemaStr, SchemaRef),
                    SchemaPair(SchemaStr, SchemaUnit)
                ),
                SchemaUnit
            )
        );

        schm.find(glob, u).and_then(|or| {
            match or {
                Or::First(s) =>
                match s.as_str() {
                    "cls" => Some(Act::Clear),
                    "key" => Some(Act::GetKey(None)),
                    "nl" => Some(Act::Nl),
                    "trc" => Some(Act::Trc),
                    "say" => Some(Act::Say(
                        ui::Say {
                            msg: Unit::Ref(vec!["msg".into()]),
                            shrt: None,
                            nl: false,
                            mode: ui::SayMode::Norm
                        }
                    )),
                    "say.fmt" => Some(Act::Say(
                        ui::Say {
                            msg: Unit::Ref(vec!["msg".into()]),
                            shrt: None,
                            nl: false,
                            mode: ui::SayMode::Fmt
                        }
                    )),
                    "res.cli" => Some(Act::GetRes(GetRes::Cli, vec!["msg".into()])),
                    "res.gfx" => Some(Act::GetRes(GetRes::Gfx, vec!["msg".into()])),
                    "res.cli.lst" => Some(Act::GetRes(GetRes::ListCli, vec!["msg".into()])),
                    "res.gfx.lst" => Some(Act::GetRes(GetRes::ListGfx, vec!["msg".into()])),
                    _ => None
                },
                Or::Second(or) =>
                    match or {
                        Or::First(or) =>
                        match or {
                            Or::First((s, path)) =>
                                match s.as_str() {
                                    "key" => Some(Act::GetKey(Some(path))),
                                    "say" => Some(Act::Say(
                                        ui::Say {
                                            msg: Unit::Ref(path),
                                            shrt: None,
                                            nl: false,
                                            mode: ui::SayMode::Norm
                                        }
                                    )),
                                    "say.fmt" => Some(Act::Say(
                                        ui::Say {
                                            msg: Unit::Ref(path),
                                            shrt: None,
                                            nl: false,
                                            mode: ui::SayMode::Fmt
                                        }
                                    )),
                                    "res.cli" => Some(Act::GetRes(GetRes::Cli, path)),
                                    "res.gfx" => Some(Act::GetRes(GetRes::Gfx, path)),
                                    "res.cli.lst" => Some(Act::GetRes(GetRes::ListCli, path)),
                                    "res.gfx.lst" => Some(Act::GetRes(GetRes::ListGfx, path)),
                                    _ => None
                                },
                            Or::Second((s, msg)) =>
                                match s.as_str() {
                                    "say" => Some(Act::Say(
                                        ui::Say {
                                            msg,
                                            shrt: None,
                                            nl: false,
                                            mode: ui::SayMode::Norm
                                        }
                                    )),
                                    "say.fmt" => Some(Act::Say(
                                        ui::Say {
                                            msg,
                                            shrt: None,
                                            nl: false,
                                            mode: ui::SayMode::Fmt
                                        }
                                    )),
                                    "inp" => Some(Act::Inp(
                                        ui::Inp {
                                            pmt: msg.as_str()?,
                                            prs: false,
                                            sct: false,
                                            out: vec!["msg".into()]
                                        }
                                    )),
                                    "set.res.cli" | "set.res.gfx" => {
                                        let res = msg.as_pair().into_iter().filter_map(|(u0, u1)| Some((u0.as_int()? as usize, u1.as_int()? as usize))).next()?;

                                        match s.as_str() {
                                            "set.res.cli" => Some(Act::SetRes(SetRes::Cli, res)),
                                            "set.res.gfx" => Some(Act::SetRes(SetRes::Gfx, res)),
                                            _ => None
                                        }
                                    } 
                                    _ => None
                                }
                        },
                        Or::Second(u) => {
                            if let Some(inp) = ui::Inp::from_unit(glob, &u) {
                                return Some(Act::Inp(inp));
                            }

                            if let Some(put) = ui::Put::from_unit(glob, &u) {
                                return Some(Act::Put(put));
                            }

                            if let Some(say) = ui::Say::from_unit(glob, &u) {
                                return Some(Act::Say(say));
                            }

                            if let Some(img) = ui::Img::from_unit(glob, &u) {
                                return Some(Act::Img(img));
                            }

                            if let Some(spr) = ui::Sprite::from_unit(glob, &u) {
                                return Some(Act::Spr(spr));
                            }

                            if let Some(vid) = ui::Video::from_unit(glob, &u) {
                                return Some(Act::Vid(vid));
                            }

                            if let Some(win) = ui::Win::from_unit(glob, &u) {
                                return Some(Act::Win(win));
                            }
                            None
                        }
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

        Font::from_unit(u, u).map(|font| term.font = font);

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

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(acts) = self.act.clone() {
            let mut out_u = msg.msg.clone();

            for act in acts {
                out_u = act.act(self, &msg, out_u, kern)?;
            }

            self.flush(kern)?;
            return Ok(Some(kern.msg(&msg.ath, out_u)?));
        } else {
            if let Some(_msg) = Unit::find_ref(vec!["msg".into()].into_iter(), &msg.msg) {
                let mut out_u = msg.msg.clone();

                let act = Act::Say(ui::Say {
                    msg: _msg,
                    shrt: None,
                    nl: false,
                    mode: ui::SayMode::Norm
                });
                out_u = act.act(self, &msg, out_u, kern)?;

                self.flush(kern)?;
                return Ok(Some(kern.msg(&msg.ath, out_u)?));
            }
        }

        Ok(Some(msg))
    }
}
