use rand::{rngs::StdRng, SeedableRng, RngCore};

use crate::driver::{DispErr, Disp, Rnd, RndErr};


pub struct StubDisp;

impl Disp for StubDisp {
    fn res(&self) -> Result<(usize, usize), DispErr> {
        Ok((0, 0))
    }

    fn px(&mut self, _px: u32, _x: usize, _y: usize) -> Result<(), DispErr> {
        Ok(())
    }

    fn blk(&mut self, _pos: (i32, i32), _img_size: (usize, usize), _src: u32, _img: &[u32]) -> Result<(), DispErr> {
        Ok(())
    }

    fn fill(&mut self, _f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), DispErr> {
        Ok(())
    }

    fn mouse(&mut self, _block: bool) -> Result<Option<super::Mouse>, DispErr> {
        Ok(None)
    }
}

pub struct PRng;

impl Rnd for PRng {
    fn get_bytes(&mut self, buf: &mut [u8]) -> Result<(), RndErr> {
        let mut rng = StdRng::from_seed([1; 32]);

        rng.fill_bytes(buf);
        Ok(())
    }
}
