use alloc::vec::Vec;
use alloc::vec;
use alloc::format;

use crate::vnix::utils;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::{Serv, ServHlr, ServErr};
use crate::vnix::core::kern::KernErr;


pub struct GFX2D {
    fill: Option<u32>
}

impl Default for GFX2D {
    fn default() -> Self {
        GFX2D {
            fill: None
        }
    }
}

impl ServHlr for GFX2D {
    fn inst(msg: Msg, _serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = GFX2D::default();

        // config instance
        let e = msg.msg.find_str(&mut vec!["fill".into()].iter()).map(|col| {
            if col.starts_with("#") {
                let v = <u32>::from_str_radix(&col[1..7], 16)
                    .map_err(|_| KernErr::ServErr(ServErr::NotValidUnit))?
                    .to_le();

                inst.fill.replace(v);
                return Ok(());
            }
            Err(KernErr::ServErr(ServErr::NotValidUnit))
        });

        if let Some(e) = e {
            e?;
        }

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some(col) = self.fill {
            let res = serv.kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

            let img: Vec::<Unit> = (0..res.0*res.1).map(|_| Unit::Int(col as i32)).collect();
            let img_s = format!("{}", Unit::Lst(img));

            let img0 = utils::compress(img_s.as_str())?;
            let img_out = utils::compress(img0.as_str())?;

            let m = Unit::Map(vec![
                (Unit::Str("img".into()), Unit::Str(img_out.into())),
            ]);

            return Ok(Some(serv.kern.msg(&msg.ath, m)?))
        }

        Ok(None)
    }
}
