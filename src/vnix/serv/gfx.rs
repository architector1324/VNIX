use alloc::vec;
use spin::Mutex;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils;
use crate::vnix::core::unit::Or;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::unit::FromUnit;
use crate::vnix::core::serv::ServHelpTopic;
use crate::vnix::core::serv::ServHlrAsync;
use crate::vnix::core::unit::{Schema, SchemaInt, SchemaMapEntry, SchemaOr, SchemaPair, SchemaStr};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::kern::KernErr;
use crate::vnix::core::serv::{Serv, ServHlr};


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
            FillRes::Full => kern.drv.disp.res().map_err(|e| KernErr::DispErr(e))
        }
    }
}

impl FromUnit for GFX2D {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = GFX2D::default();

        // config instance
        let schm = SchemaOr(
            SchemaStr,
            SchemaMapEntry(
                Unit::Str("fill".into()),
                SchemaOr(
                    SchemaStr,
                    SchemaPair(
                        SchemaPair(SchemaInt, SchemaInt),
                        SchemaStr
                    )
                )
            )
        );

        schm.find_loc(u).map(|or| {
            match or {
                Or::First(col) => {
                    let v = utils::hex_to_u32(col.as_str())?;
                    inst.fill.replace((FillRes::Full, v))
                },
                Or::Second(or) =>
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
            }
        });

        Some(inst)
    }
}

impl ServHlr for GFX2D {
    fn help<'a>(self, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Service for rendering 2d graphics\nExample: {fill:#ff0000}@gfx.2d # fill screen with red color".into())
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
            if let Some(fill) = self.fill {
                let res = fill.0.get(&mut kern.lock())?;
                yield;

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
                                Unit::Str("fmt".into()),
                                Unit::Str("rgb.rle".into())
                            ),
                            (
                                Unit::Str("img".into()),
                                Unit::Lst(vec![
                                    Unit::Pair(
                                        Box::new(Unit::Int(fill.1 as i32)),
                                        Box::new(Unit::Int((res.0 * res.1) as i32))
                                    )
                                ])
                            )
                        ])
                    ),
                ]);
    
                return kern.lock().msg(&msg.ath, m).map(|msg| Some(msg));
            }

            Ok(None)
        };
        ServHlrAsync(Box::new(hlr))
    }
}
