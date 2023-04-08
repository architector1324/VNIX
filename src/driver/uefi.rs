use core::fmt::Write;
use core::ffi::c_void;
use core::ptr::NonNull;

use alloc::vec::Vec;
use alloc::boxed::Box;

use uefi::{Handle, Event};
use uefi::proto::console::gop::{GraphicsOutput, BltPixel, BltOp, BltRegion};
use uefi::proto::console::pointer::Pointer;
use uefi::proto::console::text::{Output,/* Input, */Key, ScanCode};
use uefi::proto::rng::{Rng, RngAlgorithmType};
use uefi::prelude::{SystemTable, Boot};
use uefi::table::boot::{OpenProtocolParams, OpenProtocolAttributes, MemoryType, EventType, Tpl, TimerTrigger};

use crate::thread;

use crate::vnix::utils::Maybe;
use crate::vnix::core::driver::{CLI, CLIErr, DispErr, DrvErr, Disp, TermKey, Time, TimeErr, Rnd, RndErr, Mem, MemErr, MemSizeUnits, Mouse, TimeAsync, Duration, TimeUnit};


pub struct UefiCLI {
    st: SystemTable<Boot>,
    cli_out_hlr: Handle,
    // cli_in_hlr: Handle,
}

pub struct UefiDisp {
    st: SystemTable<Boot>,
    disp_hlr: Handle,
    mouse_hlr: Handle,

    buffer: Vec<BltPixel>,
    res: (usize, usize)
}

pub struct UefiTime {
    st: SystemTable<Boot>,
    ticks: u64 // 1 tick = 10 ms.
}

pub struct UefiRnd {
    st: SystemTable<Boot>,
    rnd_hlr: Handle
}

pub struct UefiMem {
    st: SystemTable<Boot>
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

        let res = unsafe {
            let disp = st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: disp_hlr,
                    agent: st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DrvErr::Disp(DispErr::SetPixel))?;
    
            disp.current_mode_info().resolution()
        };

        Ok(UefiDisp {
            st,
            disp_hlr,
            mouse_hlr,
            buffer: (0..res.0*res.1).map(|_| BltPixel::new(0, 0, 0)).collect(),
            res
        })
    }
}

impl UefiTime {
    pub fn new(st: SystemTable<Boot>) -> Result<UefiTime, DrvErr> {
        Ok(UefiTime {
            st,
            ticks: 0
        })
    }

    unsafe extern "efiapi" fn timer_callback(_e: Event, ctx: Option<NonNull<c_void>>) {
        unsafe {
            if let Some(ctx) = ctx {
                let obj = core::mem::transmute::<*mut c_void, &mut Self>(ctx.as_ptr());
                obj.ticks += 1;
            }
        }
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

impl UefiMem {
    pub fn new(st: SystemTable<Boot>) -> Result<UefiMem, DrvErr> {
        Ok(UefiMem{st})
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

    fn res_list(&self) -> Result<Vec<(usize, usize)>, CLIErr> {
        let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| CLIErr::GetResolution)?;
        let out = cli.modes().map(|m| (m.columns(), m.rows())).collect();

        Ok(out)
    }

    fn set_res(&mut self, res: (usize, usize)) -> Result<(), CLIErr> {
        let mut cli = self.st.boot_services().open_protocol_exclusive::<Output>(self.cli_out_hlr).map_err(|_| CLIErr::SetResolution)?;
        let mode = cli.modes().find(|m| m.columns() == res.0 && m.rows() == res.1).ok_or(CLIErr::SetResolution)?;

        cli.set_mode(mode).map_err(|_| CLIErr::SetResolution)
    }

    fn glyth(&mut self, ch: char, pos: (usize, usize)) -> Result<(), CLIErr> {
        let cli = self.st.stdout();

        cli.set_cursor_position(pos.0, pos.1).map_err(|_| CLIErr::Write)?;
        write!(cli, "{ch}").map_err(|_| CLIErr::Write)?;

        Ok(())
    }

    fn get_key(&mut self, block: bool) -> Maybe<crate::vnix::core::driver::TermKey, CLIErr> {
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
        Ok(self.res)
    }

    fn res_list(&self) -> Result<Vec<(usize, usize)>, DispErr> {
        unsafe {
            let disp = self.st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: self.disp_hlr,
                    agent: self.st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DispErr::GetResolution)?;
    
            Ok(disp.modes().map(|m| m.info().resolution()).collect())
        }
    }

    fn set_res(&mut self, res: (usize, usize)) -> Result<(), DispErr> {
        unsafe {
            let mut disp = self.st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: self.disp_hlr,
                    agent: self.st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DispErr::GetResolution)?;
    
            let mode = disp.modes().find(|m| m.info().resolution() == res).ok_or(DispErr::SetResolution)?;
            disp.set_mode(&mode).map_err(|_| DispErr::SetResolution)?;
            
            self.buffer = (0..res.0*res.1).map(|_| BltPixel::new(0, 0, 0)).collect();
            self.res = res;

            Ok(())
        }
    }

    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr> {
        if x + self.res.0 * y >= self.res.0 * self.res.1 {
            return Err(DispErr::SetPixel)
        }

        if let Some(v) = self.buffer.get_mut(x + self.res.0 * y) {
            *v = BltPixel::new((px >> 16) as u8, (px >> 8) as u8, px as u8);
        }

        Ok(())
    }

    fn blk(&mut self, pos: (i32, i32), img_size: (usize, usize), src: u32, img: &[u32]) -> Result<(), DispErr> {
        for x in 0..img_size.0 {
            for y in 0..img_size.1 {
                if x as i32 + pos.0 >= self.res.0 as i32 || x as i32 + pos.0 < 0 || y as i32 + pos.1 >= self.res.1 as i32 || y as i32 + pos.1 < 0 {
                    continue;
                }

                let offs = ((pos.0 + x as i32) + self.res.0 as i32 * (pos.1 + y as i32)) as usize;

                if let Some(px) = img.get(x + img_size.0 * y) {
                    if *px != src {
                        if let Some(v) = self.buffer.get_mut(offs) {
                            *v = BltPixel::new((*px >> 16) as u8, (*px >> 8) as u8, *px as u8);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn fill(&mut self, f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr> {
        for x in 0..self.res.0 {
            for y in 0..self.res.1 {
                let px = f(x, y);
                if let Some(v) = self.buffer.get_mut(x + self.res.0 * y) {
                    *v = BltPixel::new((px >> 16) as u8, (px >> 8) as u8, px as u8);
                }
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), DispErr> {
        unsafe {
            let mut disp = self.st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: self.disp_hlr,
                    agent: self.st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DispErr::SetPixel)?;

            disp.blt(BltOp::BufferToVideo {
                buffer: &self.buffer,
                src: BltRegion::Full,
                dest: (0, 0),
                dims: (self.res.0, self.res.1)
            }).map_err(|_| DispErr::Flush)?;
        }

        Ok(())
    }

    fn flush_blk(&mut self, mut pos: (i32, i32), size: (usize, usize)) -> Result<(), DispErr> {
        pos.0 = pos.0.clamp(0, (self.res.0 - size.0) as i32);
        pos.1 = pos.1.clamp(0, (self.res.1 - size.1) as i32);

        unsafe {
            let mut disp = self.st.boot_services().open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: self.disp_hlr,
                    agent: self.st.boot_services().image_handle(),
                    controller: None
                },
                OpenProtocolAttributes::GetProtocol
            ).map_err(|_| DispErr::SetPixel)?;

            disp.blt(BltOp::BufferToVideo {
                buffer: &self.buffer,
                src: BltRegion::SubRectangle {
                    coords: (pos.0 as usize, pos.1 as usize),
                    px_stride: self.res.0
                },
                dest: (pos.0 as usize, pos.1 as usize),
                dims: size
            }).map_err(|_| DispErr::Flush)?;
        }

        Ok(())
    }

    fn mouse(&mut self, block: bool) -> Maybe<Mouse, DispErr> {
        let mut mouse = self.st.boot_services().open_protocol_exclusive::<Pointer>(self.mouse_hlr).map_err(|_| DispErr::GetMouseState)?;

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
    fn start(&mut self) -> Result<(), TimeErr> {
        let e = unsafe {
            let p = NonNull::new(core::mem::transmute(self as *mut UefiTime)).ok_or(TimeErr::StartTimer)?;
            self.st.boot_services().create_event(EventType::TIMER | EventType::NOTIFY_SIGNAL, Tpl::NOTIFY, Some(Self::timer_callback), Some(p)).map_err(|_| TimeErr::StartTimer)?
        };

        self.st.boot_services().set_timer(&e, TimerTrigger::Periodic(10 * 10000)).map_err(|_| TimeErr::StartTimer)?;
        Ok(())
    }

    fn wait(&mut self, dur: Duration) -> Result<(), TimeErr> {
        let mcs = match dur {
            Duration::Micro(mcs) => mcs,
            Duration::Milli(ms) => 1000 * ms,
            Duration::Seconds(sec) => 1000 * 1000 * sec
        };

        self.st.boot_services().stall(mcs);
        Ok(())
    }

    fn wait_async(&self, dur: Duration) -> TimeAsync {
        let st = unsafe {
            self.st.unsafe_clone()
        };

        thread!({
            unsafe {
                let mcs = match dur {
                    Duration::Micro(mcs) => mcs,
                    Duration::Milli(ms) => 1000 * ms,
                    Duration::Seconds(sec) => 1000 * 1000 * sec
                };

                let e = st.boot_services().create_event(EventType::TIMER, Tpl::APPLICATION, None, None).map_err(|_| TimeErr::Wait)?;
                st.boot_services().set_timer(&e, TimerTrigger::Relative(10 * mcs as u64)).map_err(|_| TimeErr::Wait)?;

                loop {
                    if st.boot_services().check_event(e.unsafe_clone()).map_err(|_| TimeErr::Wait)? {
                        return Ok(())
                    }
                    yield;
                }
            }
        })
    }

    fn uptime(&self, units: TimeUnit) -> Result<u64, TimeErr> {
        let time = match units {
            TimeUnit::Micro => self.ticks * 10 * 1000,
            TimeUnit::Milli => self.ticks * 10,
            TimeUnit::Second => self.ticks / 100,
            TimeUnit::Minute => self.ticks / (60 * 100),
            TimeUnit::Hour => self.ticks / (60 * 60 * 100),
            TimeUnit::Day => self.ticks / (24 * 60 * 60 * 100),
            TimeUnit::Week => self.ticks / (7 * 24 * 60 * 60 * 100),
            TimeUnit::Month => self.ticks / (4 * 7 * 24 * 60 * 60 * 100),
            TimeUnit::Year => self.ticks / (12 * 4 * 7 * 24 * 60 * 60 * 100)
        };
        Ok(time)
    }
}

impl Rnd for UefiRnd {
    fn get_bytes(&mut self, buf: &mut [u8]) -> Result<(), RndErr> {
        let mut rng = self.st.boot_services().open_protocol_exclusive::<Rng>(self.rnd_hlr).map_err(|_| RndErr::GetBytes)?;
        rng.get_rng(Some(RngAlgorithmType::ALGORITHM_RAW), buf).map_err(|_| RndErr::GetBytes)?;

        Ok(())
    }
}

impl Mem for UefiMem {
    fn free(&self, units: MemSizeUnits) -> Result<usize, MemErr> {
        let size = self.st.boot_services().memory_map_size();
        let mut tmp = (0..(size.map_size * size.entry_size)).map(|_| 0u8).collect::<Vec<u8>>();
        let mem = self.st.boot_services().memory_map(&mut tmp).map_err(|_| MemErr::NotEnough)?.1.filter(|m| m.ty == MemoryType::CONVENTIONAL).collect::<Vec<_>>();

        let size = mem.iter().map(|m| m.page_count * 4 * 1024).sum::<u64>() as usize;
        match units {
            MemSizeUnits::Bytes => Ok(size),
            MemSizeUnits::Kilo => Ok(size / 1024),
            MemSizeUnits::Mega => Ok(size / (1024 * 1024)),
            MemSizeUnits::Giga => Ok(size / (1024 * 1024 * 1024))
        }
    }
}
