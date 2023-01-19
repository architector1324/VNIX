pub mod amd64;

use core::fmt::Write;
use rand::{rngs::StdRng, SeedableRng, RngCore};

#[derive(Debug)]
pub enum CLIErr {
    Clear,
    Write,
    GetKey
}

#[derive(Debug)]
pub enum DispErr {
    GetResolution,
    SetPixel
}

#[derive(Debug)]
pub enum TimeErr {
    Wait
}

#[derive(Debug)]
pub enum RndErr {
    GetBytes
}

#[derive(Debug)]
pub enum DrvErr {
    HandleFault,
    CLI(CLIErr),
    Disp(DispErr),
    Time(TimeErr),
    Rnd(RndErr)
}

#[derive(Debug, PartialEq)]
pub enum TermKey {
    Esc,
    Char(char)
}

pub trait Time {
    fn wait(&mut self, mcs: usize) -> Result<(), TimeErr>;
}

pub trait CLI: Write {
    fn get_key(&mut self) -> Result<Option<TermKey>, CLIErr>;
    fn clear(&mut self) -> Result<(), CLIErr>;
}

pub trait Rnd {
    fn get_bytes(&mut self, buf: &mut [u8]) -> Result<(), RndErr>;
}

pub trait Disp {
    fn res(&self) -> Result<(usize, usize), DispErr>;
    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr>;
    fn fill(&mut self, f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr>;
}


// stub drivers
pub struct StubDisp;

impl Disp for StubDisp {
    fn res(&self) -> Result<(usize, usize), DispErr> {
        Ok((0, 0))
    }

    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr> {
        Ok(())
    }

    fn fill(&mut self, f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr> {
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
