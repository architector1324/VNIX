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
enum OutMode {
    Cli,
    Gfx,
}

#[derive(Debug, Clone)]
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
struct Font {
    glyths: Vec<(char, [u8; 16])>
}

#[derive(Debug)]
enum Get {
    CliRes,
    GfxRes
}

static mut CUR_POS: (usize, usize) = (0, 0);

#[derive(Debug)]
pub struct Term {
    mode: OutMode,

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

    font: Font
}

impl Default for Term {
    fn default() -> Self {
        Term {
            mode: OutMode::Cli,

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
            shrt: None,

            font: Font{glyths: Vec::from(SYS_FONT)}
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

    fn handle(&self, term: &mut Term, prs:bool, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
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
        // write!(kern.cli, "\r{}", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        term.print(format!("\r{}", self.pmt).as_str(), kern)?;

        loop {
            if let Some(key) = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                if let TermKey::Char(c) = key {
                    if c == '\r' || c == '\n' {
                        // writeln!(kern.cli).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        term.print("\n", kern)?;
                        break;
                    }

                    if c == '\u{8}' {
                        out.pop();
                    } else if c == '\u{3}' {
                        // writeln!(kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        term.print(format!("\r{}{out}\n", self.pmt).as_str(), kern)?;
                        return Ok(None);
                    } else if !c.is_ascii_control() {
                        write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    }

                    term.print(format!("\r{}{out}", self.pmt).as_str(), kern)?;
                    // write!(kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
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
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
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

        schm.find_deep(glob, u).map(|((w, h), or)| {
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
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
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

        schm.find_deep(glob, u).map(|or| {
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
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
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

        schm.find_deep(glob, u).map(|(x, (y, u))| {
            Some(Sprite {
                pos: (x, y),
                img: Img::from_unit(glob, &u)?
            })
        }).flatten()
    }
}

impl Term {
    fn clear_line(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        let (w, _) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

        unsafe {
            CUR_POS.0 = 0;
    
            for _ in 0..(w / 8 - 1) {
                self.print(" ", kern)?;
            }
            CUR_POS.0 = 0;
        }
        Ok(())
    }

    fn clear(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        let (_, h) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

        unsafe {
            CUR_POS.1 = 0;

            for _ in 0..(h / 16 - 1) {
                self.clear_line(kern)?;
            }

            CUR_POS.1 = 0;
        }
        Ok(())
    }

    fn print(&mut self, s: &str, kern: &mut Kern) -> Result<(), KernErr> {        
        match self.mode {
            OutMode::Cli => write!(kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?,
            OutMode::Gfx => {
                let (w, _) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                unsafe {
                    for ch in s.chars() {
                        if ch == '\n' {
                            CUR_POS.1 += 1;
                            CUR_POS.0 = 0;
                        } else if ch == '\r' {
                            self.clear_line(kern)?;
                        } else {
                            self.print_glyth(ch, (CUR_POS.0 * 8, CUR_POS.1 * 16), kern)?;
                            CUR_POS.0 += 1;
                        }
    
                        if CUR_POS.0 * 8 >= w {
                            CUR_POS.1 += 1;
                            CUR_POS.0 = 0;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn print_glyth(&self, ch: char, pos: (usize, usize), kern: &mut Kern) -> Result<(), KernErr> {
        let img = self.font.glyths.iter().find(|(_ch, _)| *_ch == ch).map_or(Err(KernErr::CLIErr(CLIErr::Write)), |(_, img)| Ok(img))?;

        for y in 0..16 {
            for x in 0..8 {
                let px = if (img[y] >> (8 - x)) & 1 == 1 {0xffffff} else {0x000000};
                kern.disp.px(px as u32, x + pos.0, y + pos.1).map_err(|e| KernErr::DispErr(e))?;
            }
        }

        Ok(())
    }

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

    fn cls(&mut self, kern: &mut Kern) -> Result<(), KernErr> {
        if self.cls {
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;
            self.clear(kern)?;
        }
        Ok(())
    }

    fn print_msg(&mut self, msg: &Msg, kern: &mut Kern) -> Result<(), KernErr> {
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
            // writeln!(kern.cli, "{}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            self.print(format!("{}\n", msg).as_str(), kern)?;
        } else {
            // write!(kern.cli, "{}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            self.print(format!("{}", msg).as_str(), kern)?;
        }

        Ok(())
    }

    fn put_char(&mut self, kern: &mut Kern) -> Result<bool, KernErr> {
        if let Some(put) = &self.put {
            let (w, h) = match self.mode {
                OutMode::Cli => kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
                OutMode::Gfx => {
                    let (w, h) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                    (w / 8, h / 16)
                }
            };

            let mut out = ".".repeat(w * h);
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            for ch in put {
                let offs = ch.pos.0 + w * (ch.pos.1 + 1);
                out.replace_range(offs..offs + 1, &ch.ch);
            }
            // write!(kern.cli, "{}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            self.print(format!("{}", out).as_str(), kern)?;

            // wait for key
            kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            return Ok(true);
        }

        Ok(false)
    }

    fn cli_hlr(&mut self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        self.cls(kern)?;
        self.print_msg(&msg, kern)?;

        if self.put_char(kern)? {
            return Ok(Some(msg));
        }

        if let Some(get) = &self.get {
            return get.handle(msg, kern);
        }

        if let Some(inp) = &self.inp {
            return inp.clone().handle(self, self.prs, msg, kern);
        }

        return Ok(Some(msg));
    }
}

impl FromUnit for Term {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
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
                                SchemaMapEntry(Unit::Str("get".into()), SchemaStr),
                                SchemaMapSecondRequire(
                                    SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit),
                                    SchemaMap(
                                        SchemaMapEntry(Unit::Str("inp".into()), SchemaStr),
                                        SchemaMapEntry(Unit::Str("mod".into()), SchemaStr),
                                    )
                                )
                            )
                        )
                    )
                )
            )
        );

        schm.find_loc(u).map(|(trc, (shrt, (cls, (nl, (prs, (get, (msg, (inp, mode))))))))| {
            trc.map(|v| inst.trc = v);
            shrt.map(|v| inst.shrt.replace(v as usize));
            cls.map(|v| inst.cls = v);
            nl.map(|v| inst.nl = v);
            prs.map(|v| inst.prs = v);
    
            inp.map(|pmt| inst.inp.replace(Inp{pmt}));

            mode.map(|mode| {
                match mode.as_str() {
                    "cli" => inst.mode = OutMode::Cli,
                    "gfx" => inst.mode = OutMode::Gfx,
                    _ => ()
                }
            });
    
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

        if let Some(put) = Vec::<PutChar>::from_unit(u, u) {
            inst.put.replace(put);
        }

        if let Some(img) = Img::from_unit(u, u) {
            inst.img.replace(img);
        }

        if let Some(spr) = Sprite::from_unit(u, u) {
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

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if self.trc {
            // writeln!(kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            self.print(format!("INFO vnix:io.term: {}\n", msg).as_str(), kern)?;
            return Ok(Some(msg))
        }

        // gfx
        if self.img.is_some() || self.spr.is_some() {
            self.img_hlr(kern)?;

            // wait for key
            kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
            self.clear(kern)?;

            return Ok(Some(msg));
        }

        // cli
        if let Some(msg) = self.cli_hlr(msg, kern)? {
            return Ok(Some(msg));
        }

        Ok(None)
    }
}


static SYS_FONT: [(char, [u8; 16]); 96] = [
    ('λ', [0, 0, 192, 32, 48, 16, 56, 56, 108, 100, 198, 194, 0, 0, 0, 0]),
    (' ', [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ('!', [0, 0, 24, 60, 60, 60, 24, 24, 24, 0, 24, 24, 0, 0, 0, 0]),
    ('"', [0, 102, 102, 102, 36, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ('#', [0, 0, 0, 108, 108, 254, 108, 108, 108, 254, 108, 108, 0, 0, 0, 0]),
    ('$', [24, 24, 124, 198, 194, 192, 124, 6, 6, 134, 198, 124, 24, 24, 0, 0]),
    ('%', [0, 0, 0, 0, 194, 198, 12, 24, 48, 96, 198, 134, 0, 0, 0, 0]),
    ('&', [0, 0, 56, 108, 108, 56, 118, 220, 204, 204, 204, 118, 0, 0, 0, 0]),
    ('\'', [0, 48, 48, 48, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ('(', [0, 0, 12, 24, 48, 48, 48, 48, 48, 48, 24, 12, 0, 0, 0, 0]),
    (')', [0, 0, 48, 24, 12, 12, 12, 12, 12, 12, 24, 48, 0, 0, 0, 0]),
    ('*', [0, 0, 0, 0, 0, 102, 60, 255, 60, 102, 0, 0, 0, 0, 0, 0]),
    ('+', [0, 0, 0, 0, 0, 24, 24, 126, 24, 24, 0, 0, 0, 0, 0, 0]),
    (',', [0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 24, 24, 48, 0, 0, 0]),
    ('-', [0, 0, 0, 0, 0, 0, 0, 254, 0, 0, 0, 0, 0, 0, 0, 0]),
    ('.', [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 24, 0, 0, 0, 0]),
    ('/', [0, 0, 0, 0, 2, 6, 12, 24, 48, 96, 192, 128, 0, 0, 0, 0]),
    ('0', [0, 0, 56, 108, 198, 198, 214, 214, 198, 198, 108, 56, 0, 0, 0, 0]),
    ('1', [0, 0, 24, 56, 120, 24, 24, 24, 24, 24, 24, 126, 0, 0, 0, 0]),
    ('2', [0, 0, 124, 198, 6, 12, 24, 48, 96, 192, 198, 254, 0, 0, 0, 0]),
    ('3', [0, 0, 124, 198, 6, 6, 60, 6, 6, 6, 198, 124, 0, 0, 0, 0]),
    ('4', [0, 0, 12, 28, 60, 108, 204, 254, 12, 12, 12, 30, 0, 0, 0, 0]),
    ('5', [0, 0, 254, 192, 192, 192, 252, 6, 6, 6, 198, 124, 0, 0, 0, 0]),
    ('6', [0, 0, 56, 96, 192, 192, 252, 198, 198, 198, 198, 124, 0, 0, 0, 0]),
    ('7', [0, 0, 254, 198, 6, 6, 12, 24, 48, 48, 48, 48, 0, 0, 0, 0]),
    ('8', [0, 0, 124, 198, 198, 198, 124, 198, 198, 198, 198, 124, 0, 0, 0, 0]),
    ('9', [0, 0, 124, 198, 198, 198, 126, 6, 6, 6, 12, 120, 0, 0, 0, 0]),
    (':', [0, 0, 0, 0, 24, 24, 0, 0, 0, 24, 24, 0, 0, 0, 0, 0]),
    (';', [0, 0, 0, 0, 24, 24, 0, 0, 0, 24, 24, 48, 0, 0, 0, 0]),
    ('<', [0, 0, 0, 6, 12, 24, 48, 96, 48, 24, 12, 6, 0, 0, 0, 0]),
    ('=', [0, 0, 0, 0, 0, 126, 0, 0, 126, 0, 0, 0, 0, 0, 0, 0]),
    ('>', [0, 0, 0, 96, 48, 24, 12, 6, 12, 24, 48, 96, 0, 0, 0, 0]),
    ('?', [0, 0, 124, 198, 198, 12, 24, 24, 24, 0, 24, 24, 0, 0, 0, 0]),
    ('@', [0, 0, 0, 124, 198, 198, 222, 222, 222, 220, 192, 124, 0, 0, 0, 0]),
    ('A', [0, 0, 16, 56, 108, 198, 198, 254, 198, 198, 198, 198, 0, 0, 0, 0]),
    ('B', [0, 0, 252, 102, 102, 102, 124, 102, 102, 102, 102, 252, 0, 0, 0, 0]),
    ('C', [0, 0, 60, 102, 194, 192, 192, 192, 192, 194, 102, 60, 0, 0, 0, 0]),
    ('D', [0, 0, 248, 108, 102, 102, 102, 102, 102, 102, 108, 248, 0, 0, 0, 0]),
    ('E', [0, 0, 254, 102, 98, 104, 120, 104, 96, 98, 102, 254, 0, 0, 0, 0]),
    ('F', [0, 0, 254, 102, 98, 104, 120, 104, 96, 96, 96, 240, 0, 0, 0, 0]),
    ('G', [0, 0, 60, 102, 194, 192, 192, 222, 198, 198, 102, 58, 0, 0, 0, 0]),
    ('H', [0, 0, 198, 198, 198, 198, 254, 198, 198, 198, 198, 198, 0, 0, 0, 0]),
    ('I', [0, 0, 60, 24, 24, 24, 24, 24, 24, 24, 24, 60, 0, 0, 0, 0]),
    ('J', [0, 0, 30, 12, 12, 12, 12, 12, 204, 204, 204, 120, 0, 0, 0, 0]),
    ('K', [0, 0, 230, 102, 102, 108, 120, 120, 108, 102, 102, 230, 0, 0, 0, 0]),
    ('L', [0, 0, 240, 96, 96, 96, 96, 96, 96, 98, 102, 254, 0, 0, 0, 0]),
    ('M', [0, 0, 198, 238, 254, 254, 214, 198, 198, 198, 198, 198, 0, 0, 0, 0]),
    ('N', [0, 0, 198, 230, 246, 254, 222, 206, 198, 198, 198, 198, 0, 0, 0, 0]),
    ('O', [0, 0, 124, 198, 198, 198, 198, 198, 198, 198, 198, 124, 0, 0, 0, 0]),
    ('P', [0, 0, 252, 102, 102, 102, 124, 96, 96, 96, 96, 240, 0, 0, 0, 0]),
    ('Q', [0, 0, 124, 198, 198, 198, 198, 198, 198, 214, 222, 124, 12, 14, 0, 0]),
    ('R', [0, 0, 252, 102, 102, 102, 124, 108, 102, 102, 102, 230, 0, 0, 0, 0]),
    ('S', [0, 0, 124, 198, 198, 96, 56, 12, 6, 198, 198, 124, 0, 0, 0, 0]),
    ('T', [0, 0, 126, 126, 90, 24, 24, 24, 24, 24, 24, 60, 0, 0, 0, 0]),
    ('U', [0, 0, 198, 198, 198, 198, 198, 198, 198, 198, 198, 124, 0, 0, 0, 0]),
    ('V', [0, 0, 198, 198, 198, 198, 198, 198, 198, 108, 56, 16, 0, 0, 0, 0]),
    ('W', [0, 0, 198, 198, 198, 198, 214, 214, 214, 254, 238, 108, 0, 0, 0, 0]),
    ('X', [0, 0, 198, 198, 108, 124, 56, 56, 124, 108, 198, 198, 0, 0, 0, 0]),
    ('Y', [0, 0, 102, 102, 102, 102, 60, 24, 24, 24, 24, 60, 0, 0, 0, 0]),
    ('Z', [0, 0, 254, 198, 134, 12, 24, 48, 96, 194, 198, 254, 0, 0, 0, 0]),
    ('[', [0, 0, 60, 48, 48, 48, 48, 48, 48, 48, 48, 60, 0, 0, 0, 0]),
    ('\\', [0, 0, 0, 128, 192, 224, 112, 56, 28, 14, 6, 2, 0, 0, 0, 0]),
    (']', [0, 0, 60, 12, 12, 12, 12, 12, 12, 12, 12, 60, 0, 0, 0, 0]),
    ('^', [16, 56, 108, 198, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ('_', [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0]),
    ('`', [48, 48, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ('a', [0, 0, 0, 0, 0, 120, 12, 124, 204, 204, 204, 118, 0, 0, 0, 0]),
    ('b', [0, 0, 224, 96, 96, 120, 108, 102, 102, 102, 102, 124, 0, 0, 0, 0]),
    ('c', [0, 0, 0, 0, 0, 124, 198, 192, 192, 192, 198, 124, 0, 0, 0, 0]),
    ('d', [0, 0, 28, 12, 12, 60, 108, 204, 204, 204, 204, 118, 0, 0, 0, 0]),
    ('e', [0, 0, 0, 0, 0, 124, 198, 254, 192, 192, 198, 124, 0, 0, 0, 0]),
    ('f', [0, 0, 56, 108, 100, 96, 240, 96, 96, 96, 96, 240, 0, 0, 0, 0]),
    ('g', [0, 0, 0, 0, 0, 118, 204, 204, 204, 204, 204, 124, 12, 204, 120, 0]),
    ('h', [0, 0, 224, 96, 96, 108, 118, 102, 102, 102, 102, 230, 0, 0, 0, 0]),
    ('i', [0, 0, 24, 24, 0, 56, 24, 24, 24, 24, 24, 60, 0, 0, 0, 0]),
    ('j', [0, 0, 6, 6, 0, 14, 6, 6, 6, 6, 6, 6, 102, 102, 60, 0]),
    ('k', [0, 0, 224, 96, 96, 102, 108, 120, 120, 108, 102, 230, 0, 0, 0, 0]),
    ('l', [0, 0, 56, 24, 24, 24, 24, 24, 24, 24, 24, 60, 0, 0, 0, 0]),
    ('m', [0, 0, 0, 0, 0, 236, 254, 214, 214, 214, 214, 198, 0, 0, 0, 0]),
    ('n', [0, 0, 0, 0, 0, 220, 102, 102, 102, 102, 102, 102, 0, 0, 0, 0]),
    ('o', [0, 0, 0, 0, 0, 124, 198, 198, 198, 198, 198, 124, 0, 0, 0, 0]),
    ('p', [0, 0, 0, 0, 0, 220, 102, 102, 102, 102, 102, 124, 96, 96, 240, 0]),
    ('q', [0, 0, 0, 0, 0, 118, 204, 204, 204, 204, 204, 124, 12, 12, 30, 0]),
    ('r', [0, 0, 0, 0, 0, 220, 118, 102, 96, 96, 96, 240, 0, 0, 0, 0]),
    ('s', [0, 0, 0, 0, 0, 124, 198, 96, 56, 12, 198, 124, 0, 0, 0, 0]),
    ('t', [0, 0, 16, 48, 48, 252, 48, 48, 48, 48, 54, 28, 0, 0, 0, 0]),
    ('u', [0, 0, 0, 0, 0, 204, 204, 204, 204, 204, 204, 118, 0, 0, 0, 0]),
    ('v', [0, 0, 0, 0, 0, 102, 102, 102, 102, 102, 60, 24, 0, 0, 0, 0]),
    ('w', [0, 0, 0, 0, 0, 198, 198, 214, 214, 214, 254, 108, 0, 0, 0, 0]),
    ('x', [0, 0, 0, 0, 0, 198, 108, 56, 56, 56, 108, 198, 0, 0, 0, 0]),
    ('y', [0, 0, 0, 0, 0, 198, 198, 198, 198, 198, 198, 126, 6, 12, 248, 0]),
    ('z', [0, 0, 0, 0, 0, 254, 204, 24, 48, 96, 198, 254, 0, 0, 0, 0]),
    ('{', [0, 0, 14, 24, 24, 24, 112, 24, 24, 24, 24, 14, 0, 0, 0, 0]),
    ('|', [0, 0, 24, 24, 24, 24, 0, 24, 24, 24, 24, 24, 0, 0, 0, 0]),
    ('}', [0, 0, 112, 24, 24, 24, 14, 24, 24, 24, 24, 112, 0, 0, 0, 0]),
    ('~', [0, 0, 118, 220, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
];
