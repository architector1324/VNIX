pub mod amd64;

use core::fmt::Write;

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
pub enum DrvErr {
    HandleFault,
    CLI(CLIErr),
    Disp(DispErr),
    Time(TimeErr)
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

pub trait Disp {
    fn res(&self) -> Result<(usize, usize), DispErr>;
    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr>;
    fn fill(&mut self, f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr>;
}
