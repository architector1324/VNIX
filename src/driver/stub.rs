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

    fn fill(&mut self, _f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr> {
        Ok(())
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
