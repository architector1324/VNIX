use core::fmt::Write;

use uefi::Handle;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::pointer::Pointer;
use uefi::proto::console::text::{Output,/* Input, */Key, ScanCode};
use uefi::proto::rng::{Rng, RngAlgorithmType};
use uefi::prelude::{SystemTable, Boot};
use uefi::table::boot::{OpenProtocolParams, OpenProtocolAttributes};

use crate::driver::{CLI, CLIErr, DispErr, DrvErr, Disp, TermKey, Time, TimeErr, Rnd, RndErr, Mouse};


pub struct UefiCLI {
    st: SystemTable<Boot>,
    cli_out_hlr: Handle,
    // cli_in_hlr: Handle,
}

pub struct UefiDisp {
    st: SystemTable<Boot>,
    disp_hlr: Handle,
    mouse_hlr: Handle
}

pub struct UefiTime {
    st: SystemTable<Boot>,
}

pub struct UefiRnd {
    st: SystemTable<Boot>,
    rnd_hlr: Handle
}

impl UefiCLI {
    pub fn new(st: SystemTable<Boot>) -> Result<UefiCLI, DrvErr> {
        let bt = st.boot_services();
        let cli_out_hlr = bt.get_handle_for_protocol::<Output>().map_err(|_| DrvErr::HandleFault)?;
        // let cli_in_hlr = bt.get_handle_for_protocol::<Input>().map_err(|_| DrvErr::HandleFault)?;

        Ok(UefiCLI {
            st,
            cli_out_hlr,
            // cli_in_hlr,
        })
    }
}

impl UefiDisp {
    pub fn new(st: SystemTable<Boot>) -> Result<UefiDisp, DrvErr> {
        let disp_hlr = st.boot_services().get_handle_for_protocol::<GraphicsOutput>().map_err(|_| DrvErr::HandleFault)?;
        let mouse_hlr = st.boot_services().get_handle_for_protocol::<Pointer>().map_err(|_| DrvErr::HandleFault)?;

        Ok(UefiDisp {
            st,
            disp_hlr,
            mouse_hlr
        })
    }
}

impl UefiTime {
    pub fn new(st: SystemTable<Boot>) -> Result<UefiTime, DrvErr> {
        Ok(UefiTime {
            st
        })
    }
}

impl UefiRnd {
    pub fn new(st: SystemTable<Boot>) -> Result<UefiRnd, DrvErr> {
        let rnd_hlr = st.boot_services().get_handle_for_protocol::<Rng>().map_err(|_| DrvErr::HandleFault)?;
        Ok(UefiRnd {
            st,
            rnd_hlr
        })
    }
}

impl Write for UefiCLI {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| core::fmt::Error)?;
        write!(self.st.stdout(), "{}", s)
    }
}

impl CLI for UefiCLI {
    fn res(&self) -> Result<(usize, usize), CLIErr> {
        let cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| CLIErr::GetResolution)?;
        let out = cli.current_mode().map_err(|_| CLIErr::GetResolution)?.ok_or(CLIErr::GetResolution)?;

        Ok((out.columns(), out.rows()))
    }

    fn glyth(&mut self, ch: char, pos: (usize, usize)) -> Result<(), CLIErr> {
        let cli = self.st.stdout();
        let save = cli.cursor_position();

        cli.set_cursor_position(pos.0, pos.1).map_err(|_| CLIErr::Write)?;
        write!(cli, "{ch}").map_err(|_| CLIErr::Write)?;
        cli.set_cursor_position(save.0, save.1).map_err(|_| CLIErr::Write)?;

        Ok(())
    }

    fn get_key(&mut self, block: bool) -> Result<Option<crate::driver::TermKey>, CLIErr> {
        // let mut cli = self.st.boot_services().open_protocol_exclusive::<Input>(self.cli_in_hlr).map_err(|_| CLIErr::GetKey)?;

        if block {
            unsafe {
                let cli = self.st.stdin();
                let e = cli.wait_for_key_event().unsafe_clone();
                self.st.boot_services().wait_for_event(&mut [e]).map_err(|_| CLIErr::GetKey)?;
            }
        }

        let cli = self.st.stdin();
        
        if let Some(key) = cli.read_key().map_err(|_| CLIErr::GetKey)? {
            match key {
                Key::Special(scan) => match scan {
                    ScanCode::ESCAPE => return Ok(Some(TermKey::Esc)),
                    ScanCode::UP => return Ok(Some(TermKey::Up)),
                    ScanCode::DOWN => return Ok(Some(TermKey::Down)),
                    ScanCode::LEFT => return Ok(Some(TermKey::Left)),
                    ScanCode::RIGHT => return Ok(Some(TermKey::Right)),
                    _ => return Ok(Some(TermKey::Unknown)),
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

impl Disp for UefiDisp {
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

            if x + res.0 * y >= res.0 * res.1 {
                return Err(DispErr::SetPixel)
            }

            let mut fb = disp.frame_buffer();
            fb.write_value(4 * (x + res.0 * y), px);
        }

        Ok(())
    }

    fn blk(&mut self, pos: (i32, i32), img_size: (usize, usize), src: u32, img: &[u32]) -> Result<(), DispErr> {
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

            for x in 0..img_size.0 {
                for y in 0..img_size.1 {
                    if x as i32 + pos.0 >= res.0 as i32 || x as i32 + pos.0 < 0 || y as i32 + pos.1 >= res.1 as i32 || y as i32 + pos.1 < 0 {
                        continue;
                    }

                    let offs = ((pos.0 + x as i32) + res.0 as i32 * (pos.1 + y as i32)) as usize;

                    if let Some(px) = img.get(x + img_size.0 * y) {
                        if *px != src {
                            fb.write_value(4 * offs, *px);
                        }
                    }
                }
            }
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

    fn mouse(&mut self, block: bool) -> Result<Option<Mouse>, DispErr> {
        let mut mouse = self.st.boot_services().open_protocol_exclusive::<Pointer>(self.mouse_hlr).map_err(|_| DispErr::GetMouseState)?;

        mouse.reset(false).map_err(|_| DispErr::GetMouseState)?;

        if block {
            unsafe {
                let e = mouse.wait_for_input_event().unsafe_clone();
                self.st.boot_services().wait_for_event(&mut [e]).map_err(|_| DispErr::GetMouseState)?;
            }
        }

        let mode = mouse.mode().clone();

        let state = mouse.read_state().map_err(|_| DispErr::GetMouseState)?.map(|state| {
            Mouse {
                dpos: (state.relative_movement.0, state.relative_movement.1),
                res: (mode.resolution.0 as usize, mode.resolution.1 as usize),
                click: (state.button.0, state.button.1)
            }
        });
        return Ok(state);
    }
}

impl Time for UefiTime {
    fn wait(&mut self, mcs: usize) -> Result<(), TimeErr> {
        self.st.boot_services().stall(mcs);
        Ok(())
    }
}

impl Rnd for UefiRnd {
    fn get_bytes(&mut self, buf: &mut [u8]) -> Result<(), RndErr> {
        let mut rng = self.st.boot_services().open_protocol_exclusive::<Rng>(self.rnd_hlr).map_err(|_| RndErr::GetBytes)?;
        rng.get_rng(Some(RngAlgorithmType::ALGORITHM_RAW), buf).map_err(|_| RndErr::GetBytes)?;

        Ok(())
    }
}
