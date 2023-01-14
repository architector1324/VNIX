use alloc::vec::Vec;
use alloc::vec;
use alloc::format;

use compression::prelude::{GZipEncoder, Action, EncodeExt};
use base64ct::{Base64, Encoding};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::{Serv, ServErr};
use crate::vnix::core::kern::{KernErr, Kern};


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

impl Serv for GFX2D {
    fn inst(msg: Msg, _kern: &mut Kern) -> Result<(Self, Msg), KernErr> {
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

    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(col) = self.fill {
            let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

            let img: Vec::<Unit> = (0..res.0*res.1).map(|_| Unit::Int(col as i32)).collect();
            let img_s = format!("{}", Unit::Lst(img));

            let mut enc = GZipEncoder::new();
            let compressed = img_s.as_bytes().into_iter().cloned().encode(&mut enc, Action::Finish).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::CompressionFault)?;

            let img_out = Base64::encode_string(&compressed);

            let m = vec![
                (Unit::Str("img".into()), Unit::Str(img_out.into())),
            ];

            return Ok(Some(kern.msg(&msg.ath, Unit::Map(m))?))
        }

        Ok(None)
    }
}
