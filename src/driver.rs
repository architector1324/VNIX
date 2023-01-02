use core::fmt::Arguments;

#[derive(Debug)]
pub enum CLIErr {
    Reset
}

pub trait CLI {
    fn reset(&mut self) -> Result<(), CLIErr>;
    fn print(&self, args: Arguments);
    fn println(&self, args: Arguments);
}

pub trait Disp {

}

pub mod amd64 {
    pub mod cli {
        use core::fmt::Arguments;

        pub use uefi_services::{println, print};
        use uefi::prelude::{SystemTable, Boot};
        use super::super::{CLI, CLIErr};

        pub struct Amd64CLI {
            pub st: SystemTable<Boot>
        }

        impl CLI for Amd64CLI {
            fn reset(&mut self) -> Result<(), CLIErr> {
                self.st.stdout().reset(false).map_err(|_| CLIErr::Reset)
            }

            fn print(&self, args: Arguments) {
                print!("{}", args);
            }

            fn println(&self, args: Arguments) {
                println!("{}", args);
            }
        }
    }

    pub mod disp {
        
    }
}
