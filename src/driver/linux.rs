use std::io::{self, Stdout, Read};
use std::io::Write;

use std::time::Instant;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::RngCore;

use termion;
use sysinfo;

use termion::event::{Key, Event};
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use crate::vnix::utils::Maybe;
use crate::vnix::core::driver::{CLI, CLIErr, DispErr, DrvErr, Disp, TermKey, Time, TimeErr, Rnd, RndErr, Mem, MemErr, MemSizeUnits, Mouse, TimeAsync, Duration, TimeUnit};

use crate::thread;

pub struct LinuxCLI {
    cli: RawTerminal<Stdout>,
    cli_inp: termion::AsyncReader
}

pub struct LinuxDisp;

pub struct LinuxTime {
    uptime: Instant,
}

pub struct LinuxRnd;

pub struct LinuxMem;


impl LinuxCLI {
    pub fn new() -> Result<Self, DrvErr> {
        let cli = io::stdout().into_raw_mode().map_err(|_| DrvErr::DriverFault)?;
        cli.suspend_raw_mode().map_err(|_| DrvErr::DriverFault)?;

        Ok(LinuxCLI{
            cli,
            cli_inp: termion::async_stdin()
        })
    }
}

impl LinuxDisp {
    pub fn new() -> Result<Self, DrvErr> {
        Err(DrvErr::DriverFault)
    }
}

impl LinuxTime {
    pub fn new() -> Self {
        LinuxTime {
            uptime: Instant::now()
        }
    }
}

impl CLI for LinuxCLI {
    fn res(&self) -> Result<(usize, usize), CLIErr> {
        let res = termion::terminal_size().map_err(|_| CLIErr::GetResolution)?;
        Ok((res.0 as usize, res.1 as usize))
    }

    fn res_list(&self) -> Result<Vec<(usize, usize)>, CLIErr> {
        let out = vec![self.res()?];
        Ok(out)
    }

    fn set_res(&mut self, _: (usize, usize)) -> Result<(), CLIErr> {
        Err(CLIErr::SetResolution)
    }

    fn glyth(&mut self, ch: char, pos: (usize, usize)) -> Result<(), CLIErr> {
        write!(self.cli, "{}{ch}", termion::cursor::Goto(pos.0 as u16, pos.1 as u16)).map_err(|_| CLIErr::Write)
    }

    fn get_key(&mut self, block: bool) -> Maybe<TermKey, CLIErr> {
        self.cli.activate_raw_mode().map_err(|_| CLIErr::GetKey)?;
        self.cli.flush().map_err(|_| CLIErr::GetKey)?;

        let mut key = None;

        for ev in self.cli_inp.by_ref().events() {
            key = match ev.map_err(|_| CLIErr::GetKey)? {
                Event::Key(key) => match key {
                    Key::Esc => Some(TermKey::Esc),
                    Key::Up => Some(TermKey::Up),
                    Key::Down => Some(TermKey::Down),
                    Key::Left => Some(TermKey::Left),
                    Key::Right => Some(TermKey::Right),
                    Key::Char(c) => Some(TermKey::Char(c)),
                    Key::Backspace => Some(TermKey::Char('\u{8}')),
                    _ => Some(TermKey::Unknown),
                },
                _ => None,
            };

            if key.is_some() || !block {
                break;
            }
        }

        self.cli.suspend_raw_mode().map_err(|_| CLIErr::GetKey)?;

        Ok(key)
    }

    fn clear(&mut self) -> Result<(), CLIErr> {
        write!(self.cli, "{}{}", termion::clear::All, termion::cursor::Goto(1, 1)).map_err(|_| CLIErr::Clear)?;
        self.cli.flush().map_err(|_| CLIErr::Clear)
    }
}

impl core::fmt::Write for LinuxCLI {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        write!(self.cli, "{}", s).map_err(|_| std::fmt::Error)
    }
}

impl Disp for LinuxDisp {
    fn res(&self) -> Result<(usize, usize), DispErr> {
        todo!()
    }

    fn res_list(&self) -> Result<Vec<(usize, usize)>, DispErr> {
        todo!()
    }

    fn set_res(&mut self, res: (usize, usize)) -> Result<(), DispErr> {
        todo!()
    } 

    fn mouse(&mut self, block: bool) -> Maybe<Mouse, DispErr> {
        todo!()
    }

    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr> {
        todo!()
    }

    fn blk(&mut self, pos: (i32, i32), img_size: (usize, usize), src: u32, img: &[u32]) -> Result<(), DispErr> {
        todo!()
    }

    fn fill(&mut self, f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), DispErr> {
        todo!()
    }

    fn flush_blk(&mut self, pos: (i32, i32), size: (usize, usize)) -> Result<(), DispErr> {
        todo!()
    }
}

impl Time for LinuxTime {
    fn start(&mut self) -> Result<(), TimeErr> {
        Ok(())
    }

    fn wait(&mut self, dur: Duration) -> Result<(), TimeErr> {
        let dur = match dur {
            Duration::Micro(mcs) => std::time::Duration::from_micros(mcs as u64),
            Duration::Milli(ms) => std::time::Duration::from_millis(ms as u64),
            Duration::Seconds(sec) => std::time::Duration::from_secs(sec as u64)
        };

        std::thread::sleep(dur);
        Ok(())
    }

    fn wait_async(&self, dur: Duration) -> TimeAsync {
        thread!({
            let dur = match dur {
                Duration::Micro(mcs) => std::time::Duration::from_micros(mcs as u64),
                Duration::Milli(ms) => std::time::Duration::from_millis(ms as u64),
                Duration::Seconds(sec) => std::time::Duration::from_secs(sec as u64)
            };

            let timer = Instant::now();

            loop {
                if timer.elapsed() >= dur {
                    return Ok(())
                }
                yield;
            }
        })
    }

    fn uptime(&self, units: TimeUnit) -> Result<u128, TimeErr> {
        let time = match units {
            TimeUnit::Micro => self.uptime.elapsed().as_micros(),
            TimeUnit::Milli => self.uptime.elapsed().as_millis(),
            TimeUnit::Second => self.uptime.elapsed().as_secs() as u128,
            TimeUnit::Minute => self.uptime.elapsed().as_secs() as u128 / 60,
            TimeUnit::Hour => self.uptime.elapsed().as_secs() as u128 / (60 * 60),
            TimeUnit::Day => self.uptime.elapsed().as_secs() as u128 / (24 * 60 * 60),
            TimeUnit::Week => self.uptime.elapsed().as_secs() as u128 / (7 * 24 * 60 * 60),
            TimeUnit::Month => self.uptime.elapsed().as_secs() as u128 / (4 * 7 * 24 * 60 * 60),
            TimeUnit::Year => self.uptime.elapsed().as_secs() as u128 / (12 * 4 * 7 * 24 * 60 * 60)
        };
        Ok(time)
    }
}

impl Rnd for LinuxRnd {
    fn get_bytes(&mut self, buf: &mut [u8]) -> Result<(), RndErr> {
        let mut rng = StdRng::from_entropy();
        rng.fill_bytes(buf);
        Ok(())
    }
}

impl Mem for LinuxMem {
    fn free(&self, units: MemSizeUnits) -> Result<usize, MemErr> {
        let size = sysinfo::System::new_all().free_memory() as usize;

        match units {
            MemSizeUnits::Bytes => Ok(size),
            MemSizeUnits::Kilo => Ok(size / 1024),
            MemSizeUnits::Mega => Ok(size / (1024 * 1024)),
            MemSizeUnits::Giga => Ok(size / (1024 * 1024 * 1024))
        }
    }
}
