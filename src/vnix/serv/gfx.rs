use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;

use crate::vnix::core::kern::Kern;
use crate::vnix::core::unit::FromUnit;
use crate::vnix::core::unit::Schema;
use crate::vnix::core::unit::SchemaUnit;
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
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut inst = GFX2D::default();

        // config instance
        let mut col_s = None;

        let mut col_s2 = None;
        let mut w = None;
        let mut h = None;

        let mut schm = Schema::Unit(
            SchemaUnit::Map(vec![(
                Schema::Value(Unit::Str("fill".into())),
                Schema::Or((
                    Box::new(Schema::Unit(SchemaUnit::Str(&mut col_s))),
                    Box::new(Schema::Unit(SchemaUnit::Pair((
                        Box::new(Schema::Unit(SchemaUnit::Pair((
                            Box::new(Schema::Unit(SchemaUnit::Int(&mut w))),
                            Box::new(Schema::Unit(SchemaUnit::Int(&mut h)))
                        )))),
                        Box::new(Schema::Unit(SchemaUnit::Str(&mut col_s2)))
                    ))))
                ))
            )])
        );

        schm.find(u);

        if let Some(col) = col_s {
                let v = utils::hex_to_u32(col.as_str())?;
                inst.fill.replace((FillRes::Full, v));
        }

        if let Some(((w, h), col)) = w.iter().filter_map(|w| Some(((*w, h?), col_s2.clone()?))).next() {
            let v = utils::hex_to_u32(col.as_str())?;
            inst.fill.replace((FillRes::Custom(w as usize, h as usize), v));
        }

        Some(inst)
    }
}

impl ServHlr for GFX2D {
    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(fill) = &self.fill {
            let res = fill.0.get(kern)?;

            let img: Vec::<Unit> = (0..res.0*res.1).map(|_| Unit::Int(fill.1 as i32)).collect();
            let img_s = format!("{}", Unit::Lst(img));

            let img0 = utils::compress(img_s.as_str())?;
            let img_out = utils::compress(img0.as_str())?;

            let m = Unit::Map(vec![
                (
                    Unit::Str("img".into()),
                    Unit::Pair((
                        Box::new(Unit::Pair((
                            Box::new(Unit::Int(res.0 as i32)),
                            Box::new(Unit::Int(res.1 as i32))
                        ))),
                        Box::new(Unit::Str(img_out.into()))
                    ))
                ),
            ]);

            return Ok(Some(kern.msg(&msg.ath, m)?))
        }

        Ok(None)
    }
}
