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
    fn act(self, term: &mut Term, msg: &Msg, kern: &mut Kern) -> Result<Option<Unit>, KernErr>;
}

#[derive(Debug, Clone)]
enum Act {
    Clear,
    Nl,
    GetKey(Option<Vec<String>>),
    Trc,
    GetCLiRes(Vec<String>),
    GetGfxRes(Vec<String>),
    Say(ui::Say),
    Inp(ui::Inp),
    Put(ui::Put),
    Img(ui::Img),
    Spr(ui::Sprite),
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
    fn act(self, term: &mut Term, msg: &Msg, kern: &mut Kern) -> Result<Option<Unit>, KernErr> {
        match self {
            Act::Clear => term.clear(kern)?,
            Act::Nl => term.print("\n", kern)?,
            Act::Trc => term.print(format!("{}", msg).as_str(), kern)?,
            Act::Say(say) => return say.act(term, msg, kern),
            Act::GetKey(may_path) => {
                if let Some(key) = term.get_key(kern)? {
                    if let Some(path) = may_path {
                        let u = Unit::merge_ref(path.into_iter(), Unit::Str(format!("{}", key)), Unit::Map(Vec::new()));
                        return Ok(u);
                    }
                }
            },
            Act::GetCLiRes(path) => {
                let res = kern.cli.res().map_err(|e| KernErr::CLIErr(e))?;
                let u = Unit::Pair(
                    Box::new(Unit::Int(res.0 as i32)),
                    Box::new(Unit::Int(res.1 as i32))
                );

                let u = Unit::merge_ref(path.into_iter(), u, Unit::Map(Vec::new()));
                return Ok(u);
            },
            Act::GetGfxRes(path) => {
                let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                let u = Unit::Pair(
                    Box::new(Unit::Int(res.0 as i32)),
                    Box::new(Unit::Int(res.1 as i32))
                );

                let u = Unit::merge_ref(path.into_iter(), u, Unit::Map(Vec::new()));
                return Ok(u);
            },
            Act::Inp(inp) => return inp.act(term, msg, kern),
            Act::Put(put) => return put.act(term, msg, kern),
            Act::Img(img) => return img.act(term, msg, kern),
            Act::Spr(spr) => return spr.act(term, msg, kern),
            Act::Win(win) => return win.act(term, msg, kern)
        }
        Ok(None)
    }
}

impl Term {
    fn print_glyth(&mut self, ch: char, pos: (usize, usize), kern: &mut Kern) -> Result<(), KernErr> {
        let img = self.font.glyths.iter().find(|(_ch, _)| *_ch == ch).map_or(Err(KernErr::CLIErr(CLIErr::Write)), |(_, img)| Ok(img))?;

        for y in 0..16 {
            for x in 0..8 {
                let px = if (img[y] >> (8 - x)) & 1 == 1 {0xffffff} else {0x000000};
                kern.disp.px(px as u32, x + pos.0, y + pos.1).map_err(|e| KernErr::DispErr(e))?;
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
                        self.print_glyth(' ', (kern.term.pos.0 * 8, kern.term.pos.1 * 16), kern)?;
                    } else {
                        self.print_glyth(ch, (kern.term.pos.0 * 8, kern.term.pos.1 * 16), kern)?;
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

    fn input(&mut self, secret: bool, kern: &mut Kern) -> Result<String, KernErr> {
        let mut out = String::new();

        let save_cur = kern.term.pos.clone();

        loop {
            if let Some(key) = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                if let TermKey::Char(c) = key {
                    if c == '\r' || c == '\n' {
                        break;
                    } else if c == '\u{8}' && kern.term.pos.0 > save_cur.0 {
                        out.pop();
                        self.print(format!("{}", c).as_str(), kern)?;
                    } else if !c.is_ascii_control() {
                        write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        if !secret {
                            self.print(format!("{}", c).as_str(), kern)?;
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
                let (_, h) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                kern.term.pos.1 = 0;

                for _ in 0..(h / 16 - 1) {
                    self.clear_line(kern)?;
                    kern.term.pos.1 += 1;
                }

                kern.term.pos.1 = 0;

                Ok(())
            }
        }
    }

    fn get_key(&mut self, kern: &mut Kern) -> Result<Option<TermKey>, KernErr> {
        let key = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
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
                            msg: Unit::find_ref(vec!["msg".into()].into_iter(), glob)?,
                            shrt: None,
                            nl: false
                        }
                    )),
                    "res.cli" => Some(Act::GetCLiRes(vec!["msg".into()])),
                    "res.gfx" => Some(Act::GetGfxRes(vec!["msg".into()])),
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
                                            msg: Unit::find_ref(path.into_iter(), glob)?,
                                            shrt: None,
                                            nl: false
                                        }
                                    )),
                                    "res.cli" => Some(Act::GetCLiRes(path)),
                                    "res.gfx" => Some(Act::GetGfxRes(path)),
                                    _ => None
                                },
                            Or::Second((s, msg)) =>
                                match s.as_str() {
                                    "say" => Some(Act::Say(
                                        ui::Say {
                                            msg,
                                            shrt: None,
                                            nl: false
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
        let mut out_u: Option<Unit> = None;

        if let Some(acts) = self.act.clone() {
            for act in acts {
                act.act(self, &msg, kern)?.map(|u| {
                    out_u = out_u.clone().map_or(Some(u.clone()), |out_u| Some(out_u.merge(u)))
                });
            }
        } else {
            if let Some(_msg) = Unit::find_ref(vec!["msg".into()].into_iter(), &msg.msg) {
                let act = Act::Say(ui::Say {
                    msg: _msg,
                    shrt: None,
                    nl: false
                });
                out_u = act.act(self, &msg, kern)?;
            }
        }

        if let Some(u) = out_u {
            return Ok(Some(kern.msg(&msg.ath, u)?));
        }

        Ok(Some(msg))
    }
}
