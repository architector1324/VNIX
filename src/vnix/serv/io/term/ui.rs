use alloc::{vec, format};
use alloc::string::String;
use alloc::vec::Vec;


use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::KernErr;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapSecondRequire, SchemaMapEntry, SchemaBool, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMap, SchemaMapRequire, SchemaRef, SchemaPair, SchemaOr, SchemaSeq, Or, DisplayShort};

use crate::vnix::utils;

use super::TermAct;


#[derive(Debug, Clone)]
pub struct Inp {
    pub pmt: String,
    pub prs: bool,
    pub sct: bool,
    pub out: Vec<String>
}

#[derive(Debug, Clone)]
pub struct Say {
    pub msg: Unit,
    pub shrt: Option<usize>,
    pub nl: bool
}

#[derive(Debug, Clone)]
pub struct Img {
    pub size: (usize, usize),
    pub img: Vec<u32>
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub pos: (i32, i32),
    pub img: Img
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

impl FromUnit for Say {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("say".into()), SchemaUnit),
            SchemaMap(
                SchemaMapEntry(Unit::Str("shrt".into()), SchemaInt),
                SchemaMapEntry(Unit::Str("nl".into()), SchemaBool)
            )
        );

        schm.find_deep(glob, u).and_then(|(msg, (shrt, nl))| {
            let msg = if let Some(msg) = msg {
                msg
            } else {
                Unit::find_ref(vec!["msg".into()].into_iter(), glob)?
            };

            Some(Say {
                msg,
                shrt: shrt.map(|shrt| shrt as usize),
                nl: nl.unwrap_or(false)
            })
        })
    }
}

impl FromUnit for Img {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(
                Unit::Str("size".into()),
                SchemaPair(SchemaInt, SchemaInt)
            ),
            SchemaMapEntry(
                Unit::Str("img".into()),
                SchemaOr(
                    SchemaStr,
                    SchemaSeq(SchemaInt)
                )
            )
        );

        schm.find(glob, u).and_then(|(size, or)|{
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
                size: (size.0 as usize, size.1 as usize),
                img
            })
        })
    }
}

impl FromUnit for Sprite {
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
                Unit::Str("img".into()),
                SchemaUnit
            )
        );

        schm.find_deep(glob, u).and_then(|(pos, img)| {
            let img = Img::from_unit(glob, &img)?;

            Some(Sprite {
                pos,
                img
            })
        })
    }
}

impl TermAct for Say {
    fn act(self, term: &mut super::Term, _msg: &Msg, kern: &mut crate::vnix::core::kern::Kern) -> Result<Option<Unit>, crate::vnix::core::kern::KernErr> {
        match self.msg {
            Unit::Str(s) => term.print(format!("{}", s.replace("\\n", "\n").replace("\\r", "\r")).as_str(), kern)?,
            _ => {
                if let Some(shrt) = self.shrt {
                    term.print(format!("{}", DisplayShort(&self.msg, shrt)).as_str(), kern)?;
                } else {
                    term.print(format!("{}", self.msg).as_str(), kern)?;
                }
            }
        }

        if self.nl {
            term.print(format!("\n").as_str(), kern)?;
        }

        Ok(None)
    }
}

impl TermAct for Inp {
    fn act(self, term: &mut super::Term, _msg: &Msg, kern: &mut crate::vnix::core::kern::Kern) -> Result<Option<Unit>, crate::vnix::core::kern::KernErr> {
        term.print(self.pmt.as_str(), kern)?;
        let out = term.input(self.sct, kern)?;

        if out.is_empty() {
            return Ok(None);
        }

        let out = if self.prs {
            Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0
        } else {
            Unit::Str(out)
        };

        let u = Unit::merge_ref(self.out.into_iter(), out, Unit::Map(Vec::new()));
        Ok(u)
    }
}

impl TermAct for Img {
    fn act(self, _term: &mut super::Term, _msg: &Msg, kern: &mut crate::vnix::core::kern::Kern) -> Result<Option<Unit>, KernErr> {
        for x in 0..self.size.0 {
            for y in 0..self.size.1 {
                if let Some(px) = self.img.get(x + self.size.0 * y) {
                    kern.disp.px(*px, x, y).map_err(|e| KernErr::DispErr(e))?;
                }
            }
        }
        Ok(None)
    }
}

impl TermAct for Sprite {
    fn act(self, _term: &mut super::Term, _msg: &Msg, kern: &mut crate::vnix::core::kern::Kern) -> Result<Option<Unit>, KernErr> {
        let w = self.img.size.0;
        let h = self.img.size.1;

        for x in 0..w {
            for y in 0..h {
                if let Some(px) = self.img.img.get(x + w * y) {
                    let x_offs = (self.pos.0 - (w as i32 / 2)) as usize;
                    let y_offs = (self.pos.1 - (h as i32 / 2)) as usize;

                    kern.disp.px(*px, x + x_offs, y + y_offs).map_err(|e| KernErr::DispErr(e))?;
                }
            }
        }
        Ok(None)
    }
}