use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::vec;

use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::ServHelpTopic;
use crate::vnix::core::unit::FromUnit;
use crate::vnix::core::unit::Or;
use crate::vnix::core::unit::{Schema, SchemaInt, SchemaMapEntry, SchemaOr, SchemaPair, SchemaStr};
use crate::vnix::utils;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;


pub enum FillRes {
    Custom(usize, usize),
    Full
}

pub struct GFX2D {
    fill: Option<(FillRes, u32)>
}

impl Default for GFX2D {
    fn default() -> Self {
        GFX2D {
            fill: None
        }
    }
}

impl FillRes {
    fn get(&self, kern: &mut Kern) -> Result<(usize, usize), KernErr> {
        match self {
            FillRes::Custom(w, h) => Ok((*w, *h)),
            FillRes::Full => kern.disp.res().map_err(|e| KernErr::DispErr(e))
        }
    }
}

impl FromUnit for GFX2D {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = GFX2D::default();

        // config instance
        let schm = SchemaMapEntry(
            Unit::Str("fill".into()),
            SchemaOr(
                SchemaStr,
                SchemaPair(
                    SchemaPair(SchemaInt, SchemaInt),
                    SchemaStr
                )
            )
        );

        schm.find_loc(u).map(|or| {
            match or {
                Or::First(col) => {
                    let v = utils::hex_to_u32(col.as_str())?;
                    inst.fill.replace((FillRes::Full, v))
                },
                Or::Second(((w, h), col)) => {
                    let v = utils::hex_to_u32(col.as_str())?;
                    inst.fill.replace((FillRes::Custom(w as usize, h as usize), v))
                }
            }
        });

        Some(inst)
    }
}

impl ServHlr for GFX2D {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Service for rendering 2d graphics\nExample: {fill:#ff0000}@gfx.2d # fill screen with red color".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(fill) = &self.fill {
            let res = fill.0.get(kern)?;

            let img: Vec::<Unit> = (0..res.0*res.1).map(|_| Unit::Int(fill.1 as i32)).collect();
            let img_b = Unit::Lst(img).as_bytes();

            let img_out = utils::compress_bytes(&img_b)?;

            let m = Unit::Map(vec![
                (
                    Unit::Str("msg".into()),
                    Unit::Map(vec![
                        (
                            Unit::Str("size".into()),
                            Unit::Pair(
                                Box::new(Unit::Int(res.0 as i32)),
                                Box::new(Unit::Int(res.1 as i32))
                            )
                        ),
                        (
                            Unit::Str("img".into()),
                            Unit::Str(img_out.into())
                        )
                    ])
                ),
            ]);

            return Ok(Some(kern.msg(&msg.ath, m)?))
        }

        Ok(None)
    }
}
