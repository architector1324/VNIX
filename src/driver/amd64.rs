use core::fmt::Write;

use uefi::Handle;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::text::{Output,/* Input, */Key, ScanCode};
use uefi::prelude::{SystemTable, Boot};
use uefi::table::boot::{OpenProtocolParams, OpenProtocolAttributes};

use crate::driver::{CLI, CLIErr, DispErr, DrvErr, Disp, TermKey, Time, TimeErr};

pub struct Amd64CLI {
    st: SystemTable<Boot>,
    cli_out_hlr: Handle,
    // cli_in_hlr: Handle,
}

pub struct Amd64Disp {
    st: SystemTable<Boot>,
    disp_hlr: Handle
}

pub struct Amd64Time {
    st: SystemTable<Boot>,
}

impl Amd64CLI {
    pub fn new(st: SystemTable<Boot>) -> Result<Amd64CLI, DrvErr> {
        let bt = st.boot_services();
        let cli_out_hlr = bt.get_handle_for_protocol::<Output>().map_err(|_| DrvErr::HandleFault)?;
        // let cli_in_hlr = bt.get_handle_for_protocol::<Input>().map_err(|_| DrvErr::HandleFault)?;

        Ok(Amd64CLI {
            st,
            cli_out_hlr,
            // cli_in_hlr,
        })
    }
}

impl Amd64Disp {
    pub fn new(st: SystemTable<Boot>) -> Result<Amd64Disp, DrvErr> {
        let disp_hlr = st.boot_services().get_handle_for_protocol::<GraphicsOutput>().map_err(|_| DrvErr::HandleFault)?;

        Ok(Amd64Disp {
            st,
            disp_hlr
        })
    }
}

impl Amd64Time {
    pub fn new(st: SystemTable<Boot>) -> Result<Amd64Time, DrvErr> {
        Ok(Amd64Time {
            st
        })
    }
}

impl Write for Amd64CLI {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| core::fmt::Error)?;
        write!(cli, "{}", s)
    }
}

impl CLI for Amd64CLI {
    fn get_key(&mut self) -> Result<Option<crate::driver::TermKey>, CLIErr> {
        // let mut cli = self.st.boot_services().open_protocol_exclusive::<Input>(self.cli_in_hlr).map_err(|_| CLIErr::GetKey)?;

        unsafe {
            let cli = self.st.stdin();
            let e = cli.wait_for_key_event().unsafe_clone();
            self.st.boot_services().wait_for_event(&mut [e]).map_err(|_| CLIErr::GetKey)?;
        }

        let cli = self.st.stdin();
        
        if let Some(key) = cli.read_key().map_err(|_| CLIErr::GetKey)? {
            match key {
                Key::Special(scan) => match scan {
                    ScanCode::ESCAPE => return Ok(Some(TermKey::Esc)),
                    _ => ()
                },
                Key::Printable(c) => return Ok(Some(TermKey::Char(c.into())))
            }
        }
        Ok(None)
    }

    fn clear(&mut self) -> Result<(), CLIErr> {
        let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| CLIErr::Clear)?;
        cli.clear().map_err(|_| CLIErr::Clear)
    }
}

impl Disp for Amd64Disp {
    fn res(&self) -> Result<(usize, usize), DispErr> {
        unsafe {
            let disp = self.st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: self.disp_hlr,
                    agent: self.st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DispErr::SetPixel)?;
    
            Ok(disp.current_mode_info().resolution())
        }
    }

    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr> {
        unsafe {
            let mut disp = self.st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: self.disp_hlr,
                    agent: self.st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DispErr::SetPixel)?;

            let res = disp.current_mode_info().resolution();
            let mut fb = disp.frame_buffer();
            fb.write_value(4 * (x + res.0 * y), px);
        }

        Ok(())
    }

    fn fill(&mut self, f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr> {
        unsafe {
            let mut disp = self.st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: self.disp_hlr,
                    agent: self.st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DispErr::SetPixel)?;

            let res = disp.current_mode_info().resolution();
            let mut fb = disp.frame_buffer();

            for x in 0..res.0 {
                for y in 0..res.1 {
                    fb.write_value(4 * (x + res.0 * y), f(x, y));
                }
            }
        }

        Ok(())
    }
}

impl Time for Amd64Time {
    fn wait(&mut self, mcs: usize) -> Result<(), TimeErr> {
        self.st.boot_services().stall(mcs);
        Ok(())
    }
}
