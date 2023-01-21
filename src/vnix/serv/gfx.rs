use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;

use crate::vnix::utils;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::{Serv, ServHlr, ServErr};
use crate::vnix::core::kern::KernErr;


pub struct GFX2D {
    fill: Option<((usize, usize), u32)>
}

impl Default for GFX2D {
    fn default() -> Self {
        GFX2D {
            fill: None
        }
    }
}

impl ServHlr for GFX2D {
    fn inst(msg: Msg, serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = GFX2D::default();

        // config instance
        let e = msg.msg.find_str(&mut vec!["fill".into()].iter()).map(|col| {
            if col.starts_with("#") {
                let v = <u32>::from_str_radix(&col[1..7], 16)
                    .map_err(|_| KernErr::ServErr(ServErr::NotValidUnit))?
                    .to_le();

                let res = serv.kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                inst.fill.replace(((res.0, res.1), v));
                return Ok(());
            }
            Err(KernErr::ServErr(ServErr::NotValidUnit))
        }).map_or(Ok(None), |r| r.map(Some))?;

        msg.msg.find_pair(&mut vec!["fill".into()].iter()).iter()
            .filter_map(|(u0, u1)| Some((u0.as_pair()?, u1.as_str()?)))
            .filter_map(|((w, h), col)| Some(((w.as_int()?, h.as_int()?), col)))
            .map(|((w, h), col)| {
                if col.starts_with("#") {
                    let v = <u32>::from_str_radix(&col[1..7], 16)
                        .map_err(|_| KernErr::ServErr(ServErr::NotValidUnit))?
                        .to_le();
    
                    inst.fill.replace(((w as usize, h as usize), v));
                    return Ok(());
                }
                Err(KernErr::ServErr(ServErr::NotValidUnit))
            }).collect::<Result<(), KernErr>>()?;

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some(fill) = self.fill {
            let img: Vec::<Unit> = (0..fill.0.0*fill.0.1).map(|_| Unit::Int(fill.1 as i32)).collect();
            let img_s = format!("{}", Unit::Lst(img));

            let img0 = utils::compress(img_s.as_str())?;
            let img_out = utils::compress(img0.as_str())?;

            let m = Unit::Map(vec![
                (
                    Unit::Str("img".into()),
                    Unit::Pair((
                        Box::new(Unit::Pair((
                            Box::new(Unit::Int(fill.0.0 as i32)),
                            Box::new(Unit::Int(fill.0.1 as i32))
                        ))),
                        Box::new(Unit::Str(img_out.into()))
                    ))
                ),
            ]);

            return Ok(Some(serv.kern.msg(&msg.ath, m)?))
        }

        Ok(None)
    }
}
