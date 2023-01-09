pub mod term {
    use core::fmt::Write;

    pub use uefi_services::{println, print};
    use uefi::{prelude::{SystemTable, Boot}, proto::console::{text::{Output, Input, Key, ScanCode}, gop::GraphicsOutput}, Handle};
    use uefi::table::boot::{OpenProtocolParams, OpenProtocolAttributes};
    use crate::driver::{CLI, CLIErr, DispErr, DrvErr, Disp, Term, TermKey};

    pub struct Amd64Term {
        pub st: SystemTable<Boot>,
        cli_out_hlr: Handle,
        cli_in_hlr: Handle,
        disp_hlr: Handle
    }

    impl Amd64Term {
        pub fn new(st: SystemTable<Boot>) -> Result<Amd64Term, DrvErr> {
            let bt = st.boot_services();
            let cli_out_hlr = bt.get_handle_for_protocol::<Output>().map_err(|_| DrvErr::HandleFault)?;
            let cli_in_hlr = bt.get_handle_for_protocol::<Input>().map_err(|_| DrvErr::HandleFault)?;
            let disp_hlr = bt.get_handle_for_protocol::<GraphicsOutput>().map_err(|_| DrvErr::HandleFault)?;

            Ok(Amd64Term {
                st,
                cli_out_hlr,
                cli_in_hlr,
                disp_hlr
            })
        }
    }

    impl Write for Amd64Term {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| core::fmt::Error)?;
            write!(cli, "{}", s)
        }
    }

    impl CLI for Amd64Term {
        fn get_key(&mut self) -> Result<Option<crate::driver::TermKey>, CLIErr> {
            let mut cli = self.st.boot_services().open_protocol_exclusive::<Input>(self.cli_in_hlr).map_err(|_| CLIErr::GetKey)?;

            unsafe {
                let e = cli.wait_for_key_event().unsafe_clone();
                self.st.boot_services().wait_for_event(&mut [e]).map_err(|_| CLIErr::GetKey)?;
            }

            if let Some(key) = cli.read_key().map_err(|_| CLIErr::GetKey)? {
                match key {
                    Key::Special(scan) => match scan {
                        ScanCode::ESCAPE => return Ok(Some(TermKey::Esc)),
                        _ => ()
                    },
                    _ => ()
                }
            }
            Ok(None)
        }

        fn clear(&mut self) -> Result<(), CLIErr> {
            let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| CLIErr::Clear)?;
            cli.clear().map_err(|_| CLIErr::Clear)
        }
    }

    impl Disp for Amd64Term {
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
    }

    impl Term for Amd64Term {}
}
