use core::fmt::Display;


#[derive(Debug, Clone)]
pub enum Mode {
    Text,
    Gfx,
}

#[derive(Debug)]
pub struct TermBase {
    pos: (usize, usize),
    inp_lck: bool,
    pub mode: Mode
}


impl Default for TermBase {
    fn default() -> Self {
        TermBase {
            pos: (0, 0),
            inp_lck: false,
            mode: Mode::Gfx
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Mode::Text => write!(f, "txt"),
            Mode::Gfx => write!(f, "gfx")
        }
        
    }
}
