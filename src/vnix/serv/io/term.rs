use core::fmt::Write;

use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::string::String;
use alloc::vec::Vec;

use crate::driver::{CLIErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, DisplayShort, SchemaMapSecondRequire, SchemaMapEntry, SchemaBool, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMap, SchemaPair, SchemaOr, SchemaSeq, Or, SchemaMapRequire};

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::utils;


#[derive(Debug)]
struct Inp {
    pmt: String
}

#[derive(Debug)]
struct Img {
    size: (usize, usize),
    img: Vec<u32>
}

#[derive(Debug)]
struct Sprite {
    pos: (i32, i32),
    img: Img
}

#[derive(Debug)]
struct PutChar {
    pos: (usize, usize),
    ch: String
}

#[derive(Debug)]
enum Get {
    CliRes,
    GfxRes
}

#[derive(Debug)]
pub struct Term {
    inp: Option<Inp>,
    img: Option<Img>,
    spr: Option<Sprite>,
    put: Option<Vec<PutChar>>,
    get: Option<Get>,
    msg: Option<String>,

    nl: bool,
    cls: bool,
    trc: bool,
    prs: bool,
    shrt: Option<usize>,
}

impl Default for Term {
    fn default() -> Self {
        Term {
            inp: None,
            img: None,
            spr: None,
            put: None,
            get: None,
            msg: None,

            nl: true,
            cls: false,
            trc: false,
            prs: false,
            shrt: None
        }
    }
}

impl Inp {
    fn msg(prs:bool, s: String, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        let u = if !s.is_empty() {
            if prs {
                Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0
            } else {
                Unit::Str(s)
            }
        } else {
            Unit::None
        };

        let _msg = Unit::Map(vec![
            (Unit::Str("msg".into()), u)
        ]);

        return Ok(Some(kern.msg(&msg.ath, _msg)?));
    }

    fn handle(&self, prs:bool, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        let mut out = String::new();

        match self.pmt.as_str() {
            "key" => {
                if let Some(key) = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
                return Inp::msg(prs, out, msg, kern);
            },
            "key#async" => {
                if let Some(key) = kern.cli.get_key(false).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
                return Inp::msg(prs, out, msg, kern);
            }
            _ => ()
        }

        // input str
        write!(kern.cli, "\r{}", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

        loop {
            if let Some(key) = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                if let TermKey::Char(c) = key {
                    if c == '\r' || c == '\n' {
                        writeln!(kern.cli).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        break;
                    }
    
                    if c == '\u{8}' {
                        out.pop();
                    } else if c == '\u{3}' {
                        writeln!(kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        return Ok(None);
                    } else if !c.is_ascii_control() {
                        write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    }
    
                    write!(kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    // write!(kern.cli, "{c}").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
            }
        }

        // create msg
        return Inp::msg(prs, out, msg, kern);
    }
}

impl Get {
    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {        
        let res = match self {
            Get::CliRes => kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
            Get::GfxRes => kern.disp.res().map_err(|e| KernErr::DispErr(e))? 
        };

        let _msg = Unit::Map(vec![
            (
                Unit::Str("msg".into()),
                Unit::Pair(
                    Box::new(Unit::Int(res.0 as i32)),
                    Box::new(Unit::Int(res.1 as i32))
                )
            )
        ]);

        return Ok(Some(kern.msg(&msg.ath, _msg)?));
    }
}

impl FromUnit for Img {
    fn from_unit(u: &Unit) -> Option<Self> {
        let schm = SchemaMapEntry(
            Unit::Str("img".into()),
            SchemaPair(
                SchemaPair(SchemaInt, SchemaInt),
                SchemaOr(
                    SchemaStr,
                    SchemaSeq(SchemaInt)
                )
            )
        );

        schm.find(u).map(|((w, h), or)| {
            let img = match or {
                Or::First(s) => {
                    let img0 = utils::decompress(s.as_str()).ok()?;
                    let img_s = utils::decompress(img0.as_str()).ok()?;
                    let img_u = Unit::parse(img_s.chars()).ok()?.0.as_vec()?;

                    img_u.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect()
                },
                Or::Second(seq) => seq.into_iter().map(|e| e as u32).collect()
            };

            Some(Img {
                size: (w as usize, h as usize),
                img
            })
        }).flatten()
    }
}

impl FromUnit for Vec<PutChar> {
    fn from_unit(u: &Unit) -> Option<Self> {
        let schm = SchemaMapEntry(
            Unit::Str("put".into()),
            SchemaOr(
                SchemaPair(
                    SchemaPair(SchemaInt, SchemaInt),
                    SchemaStr
                ),
                SchemaSeq(
                    SchemaPair(
                        SchemaPair(SchemaInt, SchemaInt),
                        SchemaStr
                    )
                )
            )
        );

        schm.find(u).map(|or| {
            match or {
                Or::First(((x, y), ch)) => vec![
                    PutChar {
                        pos: (x as usize, y as usize),
                        ch
                    }
                ],
                Or::Second(seq) =>
                    seq.iter().cloned().map(|((x, y), ch)| {
                        PutChar {
                            pos: (x as usize, y as usize),
                            ch
                        }
                    }).collect()
            }
        })
    }
}

impl FromUnit for Sprite {
    fn from_unit(u: &Unit) -> Option<Self> {
        let schm = SchemaMapEntry(
            Unit::Str("spr".into()),
            SchemaMapRequire(
                SchemaMapEntry(Unit::Str("x".into()), SchemaInt),
                SchemaMapRequire(
                    SchemaMapEntry(Unit::Str("y".into()), SchemaInt),
                    SchemaUnit
                )
            )
        );

        schm.find(u).map(|(x, (y, u))| {
            Some(Sprite {
                pos: (x, y),
                img: Img::from_unit(&u)?
            })
        }).flatten()
    }
}

impl Term {
    fn img_hlr(&self, kern: &mut Kern) -> Result<(), KernErr> {
        if let Some(ref img) = self.img {
            for x in 0..img.size.0 {
                for y in 0..img.size.1 {
                    if let Some(px) = img.img.get(x + img.size.0 * y) {
                        kern.disp.px(*px, x, y).map_err(|e| KernErr::DispErr(e))?;
                    }
                }
            }
        }

        if let Some(ref spr) = self.spr {
            let w = spr.img.size.0;
            let h = spr.img.size.1;

            for x in 0..w {
                for y in 0..h {
                    if let Some(px) = spr.img.img.get(x + w * y) {
                        let x_offs = (spr.pos.0 - (w as i32 / 2)) as usize;
                        let y_offs = (spr.pos.1 - (h as i32 / 2)) as usize;

                        kern.disp.px(*px, x + x_offs, y + y_offs).map_err(|e| KernErr::DispErr(e))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn cls(&self, kern: &mut Kern) -> Result<(), KernErr> {
        if self.cls {
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;
        }
        Ok(())
    }

    fn print_msg(&self, msg: &Msg, kern: &mut Kern) -> Result<(), KernErr> {
        let msg = if let Some(ref s) = self.msg {
            format!("{}", s)
        } else if self.inp.is_some() || msg.msg.as_none().is_some() || self.cls {
            return  Ok(());
        }else if let Some(count) = self.shrt {
            format!("{}", DisplayShort(&msg.msg, count))
        } else {
            format!("{}", msg.msg)
        };

        if self.nl {
            writeln!(kern.cli, "{}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        } else {
            write!(kern.cli, "{}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        }

        Ok(())
    }

    fn put_char(&self, kern: &mut Kern) -> Result<bool, KernErr> {
        if let Some(put) = &self.put {
            let res = kern.cli.res().map_err(|e| KernErr::CLIErr(e))?;

            let mut out = ".".repeat(res.0 * res.1);
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            for ch in put {
                let offs = ch.pos.0 + res.0 * (ch.pos.1 + 1);
                out.replace_range(offs..offs + 1, &ch.ch);
            }
            write!(kern.cli, "{}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

            // wait for key
            kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            return Ok(true);
        }

        Ok(false)
    }

    fn cli_hlr(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        self.cls(kern)?;
        self.print_msg(&msg, kern)?;

        if self.put_char(kern)? {
            return Ok(Some(msg));
        }

        if let Some(get) = &self.get {
            return get.handle(msg, kern);
        }

        if let Some(inp) = &self.inp {
            return inp.handle(self.prs, msg, kern);
        }

        return Ok(Some(msg));
    }
}

impl FromUnit for Term {
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut inst = Term::default();

        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("trc".into()), SchemaBool),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("shrt".into()), SchemaInt),
                SchemaMapSecondRequire(
                    SchemaMapEntry(Unit::Str("cls".into()), SchemaBool),
                    SchemaMapSecondRequire(
                        SchemaMapEntry(Unit::Str("nl".into()), SchemaBool),
                        SchemaMapSecondRequire(
                            SchemaMapEntry(Unit::Str("prs".into()), SchemaBool),
                            SchemaMapSecondRequire(
                                SchemaMapEntry(Unit::Str("inp".into()), SchemaStr),
                                SchemaMap(
                                    SchemaMapEntry(Unit::Str("get".into()), SchemaStr),
                                    SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit)
                                )
                            )
                        )
                    )
                )
            )
        );

        schm.find(u).map(|(trc, (shrt, (cls, (nl, (prs, (inp, (get, msg)))))))| {
            trc.map(|v| inst.trc = v);
            shrt.map(|v| inst.shrt.replace(v as usize));
            cls.map(|v| inst.cls = v);
            nl.map(|v| inst.nl = v);
            prs.map(|v| inst.prs = v);
    
            inp.map(|s| inst.inp.replace(Inp{pmt: s}));
    
            get.map(|s| {
                match s.as_ref() {
                    "cli.res" => inst.get.replace(Get::CliRes),
                    "gfx.res" => inst.get.replace(Get::GfxRes),
                    _ => None
                }
            });
    
            msg.map(|u| {
                let s = match u {
                    Unit::Str(s) => format!("{}", s),
                    _ => if let Some(count) = inst.shrt {
                        format!("{}", DisplayShort(&u, count))
                    } else {
                        format!("{}", u)
                    }
                };
    
                inst.msg.replace(s)
            });
        });

        if let Some(put) = Vec::<PutChar>::from_unit(u) {
            inst.put.replace(put);
        }

        if let Some(img) = Img::from_unit(u) {
            inst.img.replace(img);
        }

        if let Some(spr) = Sprite::from_unit(u) {
            inst.spr.replace(spr);
        }

        Some(inst)
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

    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if self.trc {
            writeln!(kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            return Ok(Some(msg))
        }

        // gfx
        if self.img.is_some() || self.spr.is_some() {
            self.img_hlr(kern)?;

            // wait for key
            kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            return Ok(Some(msg));
        }

        // cli
        if let Some(msg) = self.cli_hlr(msg, kern)? {
            return Ok(Some(msg));
        }

        Ok(None)
    }
}
