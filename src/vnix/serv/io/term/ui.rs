use alloc::vec;
use alloc::string::String;
use alloc::vec::Vec;

use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapSecondRequire, SchemaMapEntry, SchemaBool, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMap, SchemaMapRequire, SchemaRef, SchemaPair, SchemaOr, SchemaSeq, Or};

use crate::vnix::utils;


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
