use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use spin::Mutex;

use crate::vnix::utils;
use crate::vnix::utils::Maybe;

use crate::{as_async, maybe, as_map_find_as_async, as_map_find_async, maybe_ok, read_async};

use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::driver::{DrvErr, Duration, TimeUnit};
use crate::vnix::core::unit::{Unit, UnitAs, UnitReadAsyncI, UnitTypeAsyncResult};


pub struct Media;

pub struct Img {
    dat: Vec<u32>,
    size: (usize, usize)
}

impl Media {
    pub async fn img(pos: (i32, i32), ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeAsyncResult<Img> {
        if let super::Mode::Text = kern.lock().term.lock().mode {
            return Ok(None)
        }
    
        // parse
        let (dat, ath) = maybe!(as_map_find_async!(msg, "img", ath, orig, kern));
    
        let it = if let Some(lst) = dat.clone().as_list() {
            let it = Rc::unwrap_or_clone(lst).into_iter();
            Box::new(it) as Box<dyn Iterator<Item = Unit>>
        } else if let Some(s) = dat.as_str() {
            // optimized units iterator from bytes
            let it = maybe!(utils::unit_compressed_iterator(&s));
            Box::new(it)
        } else {
            return Ok(None)
        };
    
        let (fmt, ath) = maybe!(as_map_find_as_async!(msg, "fmt", as_str, ath, orig, kern));
    
        let ((w, h), ath) = maybe!(as_map_find_as_async!(msg, "size", as_pair, ath, orig, kern));
        let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
        let (h, mut ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));
    
        // get image
        let img = match fmt.as_str() {
            "rgb" => {
                let mut img = Vec::with_capacity((w * h) as usize);
                for px in it {
                    let (px, _ath) = maybe!(as_async!(px, as_uint, ath, orig, kern));
                    ath = _ath;
    
                    img.push(px);
                }
                img
            },
            "rgb.rle" => {
                let mut img = Vec::with_capacity((w * h) as usize);
                for px in it {
                    let ((cnt, px), _ath) = maybe!(as_async!(px, as_pair, ath, orig, kern));
                    let (cnt, _ath) = maybe!(as_async!(cnt, as_uint, _ath, orig, kern));
                    let (px, _ath) = maybe!(as_async!(px, as_uint, _ath, orig, kern));
                    ath = _ath;
    
                    img.extend((0..cnt).map(|_| px));
                }
                img
            },
            "rgba" => todo!(),
            "rgba.rle" => todo!(),
            _ => return Ok(None)
        };
    
        // draw
        kern.lock().drv.disp.blk(pos, (w as usize, h as usize), 0, &img).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
        kern.lock().drv.disp.flush_blk(pos, (w as usize, h as usize)).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
    
        let img = Img {
            dat: img.to_vec(),
            size: (w as usize, h as usize)
        };
    
        Ok(Some((img, ath)))
    }
    
    pub async fn vid(pos: (i32, i32), ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> Maybe<Rc<String>, KernErr> {
        if let super::Mode::Text = kern.lock().term.lock().mode {
            return Ok(None)
        }
    
        // parse
        let (fms, ath) = maybe!(as_map_find_as_async!(msg, "vid", as_list, ath, orig, kern));
        let (first, ath) = maybe!(as_map_find_async!(msg, "img", ath, orig, kern));
        let (fps, ath) = as_map_find_as_async!(msg, "fps", as_uint, ath, orig, kern)?.unwrap_or((60, ath));
    
        let (blk, ath) = maybe!(as_map_find_async!(msg, "blk", ath, orig, kern));
        let (blk_size, ath) = as_map_find_as_async!(blk, "size", as_uint, ath, orig, kern)?.unwrap_or((32, ath));
        let (blk, mut ath) = maybe!(as_map_find_as_async!(blk, "blk", as_list, ath, orig, kern));
    
        // cache blocks
        let mut blk_cache = Vec::with_capacity(blk.len());
    
        for i in 0..blk.len() {
            let mut blk_rle = Vec::with_capacity((blk_size * blk_size) as usize);
    
            let _blk = blk[i].clone();
            let (blk, _ath) = maybe!(read_async!(_blk, ath, orig, kern));
            drop(_blk);
            ath = _ath;
    
            let it = if let Some(lst) = blk.clone().as_list() {
                let it = Rc::unwrap_or_clone(lst).into_iter();
                Box::new(it) as Box<dyn Iterator<Item = Unit>>
            } else if let Some(s) = blk.as_str() {
                // optimized units iterator from bytes
                let it = maybe!(utils::unit_compressed_iterator(&s));
                Box::new(it)
            } else {
                return Ok(None)
            };
    
            for dpx in it {
                let ((cnt, dpx), _ath) = maybe!(as_async!(dpx, as_pair, ath, orig, kern));
                let (cnt, _ath) = maybe!(as_async!(cnt, as_uint, _ath, orig, kern));
                let (dpx, _ath) = maybe!(as_async!(dpx, as_int, _ath, orig, kern));
                ath = _ath;
    
                blk_rle.push((cnt as u16, dpx));
            }
    
            blk_cache.push(blk_rle);
        }
    
        // render first frame
        let (mut last, mut ath) = maybe!(Self::img(pos, ath.clone(), orig.clone(), first, kern).await);
    
        // render frames
        for frame in Rc::unwrap_or_clone(fms) {
            let start = kern.lock().drv.time.uptime(TimeUnit::Milli).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;
    
            let (frame, _ath) = maybe!(as_async!(frame, as_map, ath, orig, kern));
            ath = _ath;
    
            // find block
            for (blk_pos, blk_id) in Rc::unwrap_or_clone(frame) {
                let (blk_id, _ath) = maybe!(as_async!(blk_id, as_uint, ath, orig, kern));
    
                let ((blk_x, blk_y), _ath) = maybe!(as_async!(blk_pos, as_pair, _ath, orig, kern));
                let (blk_x, _ath) = maybe!(as_async!(blk_x, as_uint, _ath, orig, kern));
                let (blk_y, _ath) = maybe!(as_async!(blk_y, as_uint, _ath, orig, kern)); 
    
                // change image
                let blk = maybe_ok!(blk_cache.get(blk_id as usize));
    
                let mut idx = 0;
                let mut blk_img = Vec::new();
                for (cnt, dpx) in blk.iter() {
                    for _ in 0..*cnt {
                        let img_x = blk_x as usize * blk_size as usize + (idx % blk_size as usize);
                        let img_y = blk_y as usize * blk_size as usize + idx / blk_size as usize;
                        let px = maybe_ok!(last.dat.get_mut(img_x + last.size.0 * img_y));
    
                        *px = (*px as i64 + *dpx as i64) as u32;
                        blk_img.push(*px);
                        idx += 1;
                    }
                }
    
                // render block
                kern.lock().drv.disp.blk((blk_x as i32 * (blk_size as i32) + pos.0, blk_y as i32 * (blk_size as i32) + pos.1), (blk_size as usize, blk_size as usize), 0x00ff00, &blk_img).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
                // kern.lock().drv.disp.flush_blk((blk_x as i32 * (blk_size as i32) + pos.0, blk_y as i32 * (blk_size as i32) + pos.1), (blk_size as usize, blk_size as usize)).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
            }
    
            kern.lock().drv.disp.flush_blk(pos, last.size).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
    
            let end = kern.lock().drv.time.uptime(TimeUnit::Milli).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;
            let elapsed = (end - start) as usize;
    
            // limit fps
            if elapsed < 900 / fps as usize {
                let _ = kern.lock().drv.time.wait_async(Duration::Milli(900 / fps as usize - elapsed)).await;
            }
        }
    
        Ok(Some(ath))
    }
    
    pub async fn spr(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> Maybe<Rc<String>, KernErr> {
        if let super::Mode::Text = kern.lock().term.lock().mode {
            return Ok(None)
        }
    
        // get pos
        let (pos, mut ath) = maybe!(as_map_find_async!(msg, "spr", ath, orig, kern));
    
        let pos = if let Some((x, y)) = pos.clone().as_pair() {
            let (x, _ath) = maybe!(as_async!(x, as_int, ath, orig, kern));
            let (y, _ath) = maybe!(as_async!(y, as_int, _ath, orig, kern));
            ath = _ath;
            (x, y)
        } else if let Some(s) = pos.as_str() {
            match s.as_str() {
                "center" => {
                    let (w, h) = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
                    (w as i32 / 2, h as i32 / 2)
                },
                _ => return Ok(None)
            }
        } else {
            return Ok(None)
        };
    
        // render video
        if let Some((_vid, ath)) = as_map_find_async!(msg, "vid", ath, orig, kern)? {
            let (img, ath) = maybe!(as_map_find_async!(_vid, "img", ath, orig, kern));
            let ((w, h), ath) = maybe!(as_map_find_as_async!(img, "size", as_pair, ath, orig, kern));
            let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
            let (h, ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));
    
            let vid_pos = (pos.0 - w as i32 / 2, pos.1 - h as i32 / 2);
    
            return Self::vid(vid_pos, ath, orig, _vid, kern).await
        }
    
        // render image
        if let Some((_img, ath)) = as_map_find_async!(msg, "img", ath, orig, kern)? {
            let ((w, h), ath) = maybe!(as_map_find_as_async!(_img, "size", as_pair, ath, orig, kern));
            let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
            let (h, ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));
    
            let img_pos = (pos.0 - w as i32 / 2, pos.1 - h as i32 / 2);
    
            let (_, ath) = maybe!(Self::img(img_pos, ath, orig, _img, kern).await);
            return Ok(Some(ath))
        }
    
        Ok(None)
    }
}
