pub mod amd64;

use core::fmt::Write;

#[derive(Debug)]
pub enum CLIErr {
    Reset,
    Write
}

#[derive(Debug)]
pub enum DispErr {
    GetResolution,
    SetPixel
}

#[derive(Debug)]
pub enum DrvErr {
    HandleFault,
    CLI(CLIErr),
    Disp(DispErr)
}

pub trait CLI: Write {
    fn reset(&mut self) -> Result<(), CLIErr>;
}

pub trait Disp {
    fn res(&self) -> Result<(usize, usize), DispErr>;
    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr>;
}

pub trait Term: CLI + Disp {}
