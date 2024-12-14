use std::io::stdout;
use std::io::Write;

use std::time::Instant;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::RngCore;

use async_trait::async_trait;

use sysinfo;
use linuxfb::Framebuffer;

use crossterm::{cursor, event, style, terminal, ExecutableCommand, QueueableCommand};

use crate::vnix::utils::Maybe;
use crate::vnix::core::driver::{CLI, CLIErr, DispErr, DrvErr, Disp, TermKey, Time, TimeErr, Rnd, RndErr, Mem, MemErr, MemSizeUnits, Mouse, Duration, TimeUnit};


pub struct LinuxCLI {
}

pub struct LinuxDisp {
    fb: Framebuffer,
    buffer: Vec<[u8; 4]>
}

pub struct LinuxTime {
    uptime: Instant,
}

pub struct LinuxRnd;

pub struct LinuxMem;


impl LinuxCLI {
    pub fn new() -> Result<Self, DrvErr> {
        Ok(LinuxCLI{
        })
    }
}

impl LinuxDisp {
    pub fn new() -> Result<Self, DrvErr> {
        let fb = Framebuffer::new("/dev/fb0").map_err(|_| DrvErr::DriverFault)?;
        let (w, h) = fb.get_size();

        let buffer = vec![[0, 0, 0, 0]; w as usize * h as usize];

        stdout().queue(terminal::Clear(terminal::ClearType::All))
                .map_err(|_| DrvErr::DriverFault)?
                .queue(cursor::MoveTo(0, 0))
                .map_err(|_| DrvErr::DriverFault)?
                .queue(cursor::Hide)
                .map_err(|_| DrvErr::DriverFault)?
                .flush()
                .map_err(|_| DrvErr::DriverFault)?;

        Ok(LinuxDisp {fb, buffer})
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
        let res = terminal::size().map_err(|_| CLIErr::GetResolution)?;
        Ok((res.0 as usize, res.1 as usize))
    }

    fn res_list(&self) -> Result<Vec<(usize, usize)>, CLIErr> {
        let out = vec![self.res()?];
        Ok(out)
    }

    fn set_res(&mut self, size: (usize, usize)) -> Result<(), CLIErr> {
        stdout().execute(terminal::SetSize(size.0 as u16, size.1 as u16)).map_err(|_| CLIErr::SetResolution)?;
        Ok(())
    }

    fn glyth(&mut self, ch: char, pos: (usize, usize)) -> Result<(), CLIErr> {
        stdout().queue(cursor::MoveTo(pos.0 as u16, pos.1 as u16))
                .map_err(|_| CLIErr::Write)?
                .queue(style::Print(ch.to_string()))
                .map_err(|_| CLIErr::Write)?
                .flush().map_err(|_| CLIErr::Write)?;

        Ok(())
    }

    fn get_key(&mut self, block: bool) -> Maybe<TermKey, CLIErr> {
        terminal::enable_raw_mode().map_err(|_| CLIErr::GetKey)?;
        stdout().flush().map_err(|_| CLIErr::GetKey)?;

        let mut key = None;

        if block {
            key = match event::read().map_err(|_| CLIErr::GetKey)? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => Some(TermKey::Esc),
                    event::KeyCode::Up => Some(TermKey::Up),
                    event::KeyCode::Down => Some(TermKey::Down),
                    event::KeyCode::Left => Some(TermKey::Left),
                    event::KeyCode::Right => Some(TermKey::Right),
                    event::KeyCode::Char(c) => Some(TermKey::Char(c)),
                    event::KeyCode::Backspace => Some(TermKey::Char('\u{8}')),
                    _ => Some(TermKey::Unknown),
                }
                _ => None
            }
        } else if event::poll(std::time::Duration::from_millis(1)).map_err(|_| CLIErr::GetKey)? {
            key = match event::read().map_err(|_| CLIErr::GetKey)? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => Some(TermKey::Esc),
                    event::KeyCode::Up => Some(TermKey::Up),
                    event::KeyCode::Down => Some(TermKey::Down),
                    event::KeyCode::Left => Some(TermKey::Left),
                    event::KeyCode::Right => Some(TermKey::Right),
                    event::KeyCode::Enter => Some(TermKey::Char('\n')),
                    event::KeyCode::Char(c) => Some(TermKey::Char(c)),
                    event::KeyCode::Backspace => Some(TermKey::Char('\u{8}')),
                    _ => Some(TermKey::Unknown),
                }
                _ => None
            }
        }

        terminal::disable_raw_mode().map_err(|_| CLIErr::GetKey)?;

        Ok(key)
    }

    fn clear(&mut self) -> Result<(), CLIErr> {
        stdout().queue(terminal::Clear(terminal::ClearType::All))
                .map_err(|_| CLIErr::Clear)?
                .queue(cursor::MoveTo(0, 0))
                .map_err(|_| CLIErr::Clear)?;

        stdout().flush().map_err(|_| CLIErr::Clear)?;
        Ok(())
    }
}

impl core::fmt::Write for LinuxCLI {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        write!(stdout(), "{}", s.replace("\n", "\r\n")).map_err(|_| std::fmt::Error)
    }
}

impl Disp for LinuxDisp {
    fn res(&self) -> Result<(usize, usize), DispErr> {
        let (w, h) = self.fb.get_size();
        Ok((w as usize, h as usize))
    }

    fn res_list(&self) -> Result<Vec<(usize, usize)>, DispErr> {
        let out = vec![self.res()?];
        Ok(out)
    }

    fn set_res(&mut self, _res: (usize, usize)) -> Result<(), DispErr> {
        Err(DispErr::SetResolution)
    } 

    fn mouse(&mut self, _block: bool) -> Maybe<Mouse, DispErr> {
        todo!()
    }

    fn px(&mut self, px: u32, x: usize, y: usize) -> Result<(), DispErr> {
        let res = self.res()?;

        if x + res.0 * y >= res.0 * res.1 {
            return Err(DispErr::SetPixel)
        }

        if let Some(v) = self.buffer.get_mut(x + res.0 * y) {
            *v = [(px >> 16) as u8, (px >> 8) as u8, px as u8, 0];
        }

        Ok(())
    }

    fn blk(&mut self, pos: (i32, i32), img_size: (usize, usize), src: u32, img: &[u32]) -> Result<(), DispErr> {
        let res = self.res()?;

        for x in 0..img_size.0 {
            for y in 0..img_size.1 {
                if x as i32 + pos.0 >= res.0 as i32 || x as i32 + pos.0 < 0 || y as i32 + pos.1 >= res.1 as i32 || y as i32 + pos.1 < 0 {
                    continue;
                }

                let offs = ((pos.0 + x as i32) + res.0 as i32 * (pos.1 + y as i32)) as usize;

                if let Some(px) = img.get(x + img_size.0 * y) {
                    if *px != src {
                        if let Some(v) = self.buffer.get_mut(offs) {
                            *v = [(*px >> 16) as u8, (*px >> 8) as u8, *px as u8, 0];
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn fill(&mut self, f: &dyn Fn(usize, usize) -> u32) -> Result<(), DispErr> {
        let res = self.res()?;

        for x in 0..res.0 {
            for y in 0..res.1 {
                let px = f(x, y);
                if let Some(v) = self.buffer.get_mut(x + res.0 * y) {
                    *v = [(px >> 16) as u8, (px >> 8) as u8, px as u8, 0];
                }
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), DispErr> {
        let mut tmp = self.fb.map().map_err(|_| DispErr::Flush)?;
        let mem = tmp.as_chunks_mut::<4>().0;

        for (i, px) in mem.iter_mut().enumerate() {
            *px = *self.buffer.get(i).ok_or(DispErr::Flush)?;
        }
        Ok(())
    }

    fn flush_blk(&mut self, mut pos: (i32, i32), size: (usize, usize)) -> Result<(), DispErr> {
        let (w, h) = self.fb.get_size();

        pos.0 = pos.0.clamp(0, (w as usize - size.0) as i32);
        pos.1 = pos.1.clamp(0, (h as usize - size.1) as i32);

        let mut tmp = self.fb.map().map_err(|_| DispErr::Flush)?;
        let mem = tmp.as_chunks_mut::<4>().0;

        for y in (pos.1 as usize)..(pos.1 as usize + size.1) {
            for x in (pos.0 as usize)..(pos.0 as usize + size.0) {
                let offs = x + y * w as usize;
                mem[offs] = self.buffer[offs]; 
            }
        }

        Ok(())
    }
}

#[async_trait(?Send)]
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

    async fn wait_async(&self, dur: Duration) -> Result<(), TimeErr> {
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
            async{}.await;
        }
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
