use core::fmt::Write;

#[derive(Debug)]
pub enum CLIErr {
    Reset,
    Write
}

pub trait CLI: Write {
    fn reset(&mut self) -> Result<(), CLIErr>;
}

pub trait Disp {

}

pub mod amd64 {
    pub mod cli {
        use core::fmt::Write;

        pub use uefi_services::{println, print};
        use uefi::prelude::{SystemTable, Boot};
        use super::super::{CLI, CLIErr};

        pub struct Amd64CLI {
            pub st: SystemTable<Boot>
        }

        impl Write for Amd64CLI {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                print!("{}", s);
                Ok(())
            }
        }

        impl CLI for Amd64CLI {
            fn reset(&mut self) -> Result<(), CLIErr> {
                self.st.stdout().reset(false).map_err(|_| CLIErr::Reset)
            }
        }
    }

    pub mod disp {
        
    }
}
