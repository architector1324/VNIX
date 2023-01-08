pub mod term {
    use core::fmt::Write;

    pub use uefi_services::{println, print};
    use uefi::{prelude::{SystemTable, Boot}, proto::console::{text::Output, gop::GraphicsOutput}, Handle};
    use crate::driver::{CLI, CLIErr, DispErr, DrvErr, Disp, Term};

    pub struct Amd64Term {
        pub st: SystemTable<Boot>,
        cli_hlr: Handle,
        disp_hlr: Handle
    }

    impl Amd64Term {
        pub fn new(st: SystemTable<Boot>) -> Result<Amd64Term, DrvErr> {
            let bt = st.boot_services();
            let cli_hlr = bt.get_handle_for_protocol::<Output>().map_err(|_| DrvErr::HandleFault)?;
            let disp_hlr = bt.get_handle_for_protocol::<GraphicsOutput>().map_err(|_| DrvErr::HandleFault)?;

            Ok(Amd64Term {
                st,
                cli_hlr,
                disp_hlr
            })
        }
    }

    impl Write for Amd64Term {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_hlr).map_err(|_| core::fmt::Error)?;
            write!(cli, "{}", s)
        }
    }

    impl CLI for Amd64Term {
        fn reset(&mut self) -> Result<(), CLIErr> {
            let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_hlr).map_err(|_| CLIErr::Reset)?;
            cli.reset(false).map_err(|_| CLIErr::Reset)
        }
    }

    impl Disp for Amd64Term {
        fn res(&self) -> Result<(usize, usize), DispErr> {
            let disp = self.st.boot_services().open_protocol_exclusive::<GraphicsOutput>(self.disp_hlr).map_err(|_| DispErr::SetPixel)?;
            Ok(disp.current_mode_info().resolution())
        }

        fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr> {
            let mut disp = self.st.boot_services().open_protocol_exclusive::<GraphicsOutput>(self.disp_hlr).map_err(|_| DispErr::SetPixel)?;

            let res = disp.current_mode_info().resolution();
            let mut fb = disp.frame_buffer();

            unsafe {
                fb.write_value(4 * (x + res.0 * y), px);
            }
            Ok(())
        }
    }

    impl Term for Amd64Term {}
}
