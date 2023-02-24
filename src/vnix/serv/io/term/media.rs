use alloc::sync::Arc;

use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::driver::DispErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaInt, SchemaStr, Schema, SchemaMapRequire, SchemaPair, SchemaOr, SchemaSeq, Or, SchemaUnit};

use crate::vnix::utils;

use super::{TermAct, Term, TermActAsync, ActMode};


trait ParseBytes: Sized {
    fn parse_bytes<'a, I>(it: I) -> Option<(Self, I)> where I: Iterator<Item = &'a u8>;
}

#[derive(Debug, Clone)]
enum ImgFmt {
    Rgb,
    Rgba,
    RgbRLE,
    RgbaRLE
}

#[derive(Debug, Clone)]
enum Pixels {
    Rgb(Vec<(u8, u8, u8)>),
    Rgba(Vec<u32>),
    RgbRLE(Vec<(usize, (u8, u8, u8))>),
    RgbaRLE(Vec<(usize, u32)>)
}

#[derive(Debug, Clone)]
pub struct Img {
    size: (usize, usize),
    img: Pixels,
    _cache: Option<Vec<u32>>
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub pos: (i32, i32),
    pub img: Img
}

impl Img {
    fn draw(&mut self, pos: (i32, i32), src: u32, kern: &mut Kern) -> Result<(), DispErr> {
        let size = self.size.clone();

        self.cache();
        kern.drv.disp.blk(pos, size, src, self._cache.as_ref().ok_or(DispErr::SetPixel)?)?;

        Ok(())
    }

    fn cache(&mut self) {
        if self._cache.is_none() {
            let img = match &self.img {
                Pixels::Rgb(dat) => dat.iter().cloned().map(|(r, g, b)| u32::from_le_bytes([r, g, b, 0])).collect(),
                Pixels::Rgba(dat) => dat.clone(),
                Pixels::RgbRLE(rle) => rle.iter().cloned().flat_map(|(cnt, (r, g, b))| (0..cnt).map(|_| u32::from_le_bytes([r, g, b, 0])).collect::<Vec<_>>()).collect(),
                Pixels::RgbaRLE(rle) => rle.iter().cloned().flat_map(|(cnt, px)| (0..cnt).map(|_| px).collect::<Vec<_>>()).collect()
            };

            self._cache = Some(img);
        }
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
            SchemaMapRequire(
                SchemaMapEntry(Unit::Str("fmt".into()), SchemaStr),
                SchemaMapEntry(
                    Unit::Str("img".into()),
                    SchemaOr(
                        SchemaStr,
                        SchemaOr(
                            SchemaSeq(
                                SchemaPair(SchemaInt, SchemaInt)
                            ),
                            SchemaSeq(SchemaInt)
                        )
                    )
                )
            )
        );

        schm.find_deep(glob, u).and_then(|((w, h), (fmt, or))| {
            let fmt = match fmt.as_str() {
                "rgb" => ImgFmt::Rgb,
                "rgba" => ImgFmt::Rgba,
                "rgb.rle" => ImgFmt::RgbRLE,
                "rgba.rle" => ImgFmt::RgbaRLE,
                _ => return None
            };

            let img = match or {
                Or::First(s) => {
                    let img0 = utils::decompress_bytes(s.as_str()).ok()?;

                    match fmt {
                        ImgFmt::Rgb => Pixels::Rgb(img0.array_chunks::<3>().map(|ch| (ch[0], ch[1], ch[2])).collect()),
                        ImgFmt::Rgba => Pixels::Rgba(img0.array_chunks::<4>().map(|ch| u32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]])).collect()),
                        ImgFmt::RgbRLE => Pixels::RgbRLE(img0.array_chunks::<6>().map(|ch| {
                            let cnt = u32::from_le_bytes([ch[0], ch[1], ch[2], 0]);
                            (cnt as usize, (ch[3], ch[4], ch[5]))
                        }).collect()),
                        ImgFmt::RgbaRLE => Pixels::RgbaRLE(img0.array_chunks::<7>().map(|ch| {
                            let cnt = u32::from_le_bytes([ch[0], ch[1], ch[2], 0]);
                            let px = u32::from_le_bytes([ch[3], ch[4], ch[5], ch[6]]);
                            (cnt as usize, px)
                        }).collect())
                    }
                },
                Or::Second(or) =>
                    match or {
                        Or::First(rle) =>
                            match fmt {
                                ImgFmt::RgbRLE => Pixels::RgbRLE(rle.into_iter().map(|(cnt, px)| {
                                    let b = px.to_le_bytes();
                                    (cnt as usize, (b[0], b[1], b[2]))
                                }).collect()),
                                ImgFmt::RgbaRLE => Pixels::RgbaRLE(rle.into_iter().map(|(cnt, px)| (cnt as usize, px as u32)).collect()),
                                _ => return None
                            }
                        Or::Second(img) =>
                            match fmt {
                                ImgFmt::Rgb => Pixels::Rgb(img.into_iter().map(|px| {
                                    let b = px.to_le_bytes();
                                    (b[0], b[1], b[2])
                                }).collect()),
                                ImgFmt::Rgba => Pixels::Rgba(img.into_iter().map(|px| px as u32).collect()),
                                _ => return None
                            }
                    }
            };

            Some(Img {
                size: (w as usize, h as usize),
                img,
                _cache: None
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
                Unit::Str("spr".into()),
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

impl TermAct for Img {
    fn act<'a>(mut self, _orig: Arc<Msg>, msg: Unit, term: Arc<Term>, kern: &'a spin::Mutex<Kern>) -> TermActAsync<'a> {
        let hlr = move || {
            let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DispErr(e))?;
            let pos = (
                (res.0 - self.size.0) as i32 / 2,
                (res.1 - self.size.1) as i32 / 2
            );
            yield;

            self.draw(pos, 0, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
            yield;

            term.flush(&ActMode::Gfx, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;

            Ok(msg)
        };
        TermActAsync(Box::new(hlr))
    }
}

impl TermAct for Sprite {
    fn act<'a>(mut self, _orig: Arc<Msg>, msg: Unit, term: Arc<Term>, kern: &'a spin::Mutex<Kern>) -> TermActAsync<'a> {
        let hlr = move || {
            let w = self.img.size.0;
            let h = self.img.size.1;

            let pos = (
                self.pos.0 - (w as i32 / 2),
                self.pos.1 - (h as i32 / 2)
            );

            self.img.draw(pos, 0, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
            yield;

            term.flush(&ActMode::Gfx, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;

            Ok(msg)
        };
        TermActAsync(Box::new(hlr))
    }
}