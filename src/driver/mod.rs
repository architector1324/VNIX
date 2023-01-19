pub mod uefi;
pub mod stub;

use core::fmt::{Write, Display};

#[derive(Debug)]
pub enum CLIErr {
    Clear,
    Write,
    GetKey,
    GetResolution
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
    Up,
    Down,
    Left,
    Right,
    Char(char)
}

pub trait Time {
    fn wait(&mut self, mcs: usize) -> Result<(), TimeErr>;
}

pub trait CLI: Write {
    fn res(&self) -> Result<(usize, usize), CLIErr>;
    fn get_key(&mut self, block: bool) -> Result<Option<TermKey>, CLIErr>;
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


impl Display for TermKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TermKey::Char(c) => write!(f, "{}", c),
            TermKey::Esc => write!(f, "ESC"),
            TermKey::Up => write!(f, "UP"),
            TermKey::Down => write!(f, "DOWN"),
            TermKey::Left => write!(f, "LEFT"),
            TermKey::Right => write!(f, "RIGHT")
        }
    }
}
