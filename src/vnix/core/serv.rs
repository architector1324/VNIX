use alloc::string::String;

use super::msg::Msg;
use super::kern::{KernErr, Kern};
use super::unit::{FromUnit, Unit};

use crate::vnix::serv::{io, etc, gfx, math, sys};


pub enum ServHelpTopic {
    Info
}

#[derive(Debug)]
pub enum ServErr {
    NotValidUnit
}

#[derive(Debug, Clone)]
pub enum ServKind {
    IOTerm,
    IOStore,
    EtcChrono,
    EtcFSM,
    GFX2D,
    MathInt,
    SysTask,
    SysUsr
}

pub enum ServInst {
    IOTerm(io::term::Term),
    IODB(io::store::Store),
    EtcChrono(etc::chrono::Chrono),
    EtcFSM(etc::fsm::FSM),
    GFX2D(gfx::GFX2D),
    MathInt(math::Int),
    SysTask(sys::task::Task),
    SysUsr(sys::usr::User)
}

#[derive(Debug, Clone)]
pub struct Serv {
    pub name: String,
    pub kind: ServKind
}

impl Serv {
    pub fn new(name: &str, kind: ServKind) -> Self {
        Serv {
            name: name.into(),
            kind
        }
    }

    pub fn inst(&self, u: &Unit) -> Option<ServInst> {
        match self.kind {
            ServKind::IOTerm => Some(ServInst::IOTerm(io::term::Term::from_unit_loc(u)?)),
            ServKind::IOStore => Some(ServInst::IODB(io::store::Store::from_unit_loc(u)?)),
            ServKind::EtcChrono => Some(ServInst::EtcChrono(etc::chrono::Chrono::from_unit_loc(u)?)),
            ServKind::EtcFSM => Some(ServInst::EtcFSM(etc::fsm::FSM::from_unit_loc(u)?)),
            ServKind::GFX2D => Some(ServInst::GFX2D(gfx::GFX2D::from_unit_loc(u)?)),
            ServKind::MathInt => Some(ServInst::MathInt(math::Int::from_unit_loc(u)?)),
            ServKind::SysTask => Some(ServInst::SysTask(sys::task::Task::from_unit_loc(u)?)),
            ServKind::SysUsr => Some(ServInst::SysUsr(sys::usr::User::from_unit_loc(u)?)),
        }
    }
}

impl FromUnit for ServInst {
    fn from_unit_loc(_u: &Unit) -> Option<Self> {
        None
    }
}

impl ServHlr for ServInst {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        match self {
            ServInst::IOTerm(inst) => inst.help(ath, topic, kern),
            ServInst::IODB(inst) => inst.help(ath, topic, kern),
            ServInst::EtcChrono(inst) => inst.help(ath, topic, kern),
            ServInst::EtcFSM(inst) => inst.help(ath, topic, kern),
            ServInst::GFX2D(inst) => inst.help(ath, topic, kern),
            ServInst::MathInt(inst) => inst.help(ath, topic, kern),
            ServInst::SysTask(inst) => inst.help(ath, topic, kern),
            ServInst::SysUsr(inst) => inst.help(ath, topic, kern),
        }
    }

    fn handle(&mut self, msg: Msg, serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        match self {
            ServInst::IOTerm(inst) => inst.handle(msg, serv, kern),
            ServInst::IODB(inst) => inst.handle(msg, serv, kern),
            ServInst::EtcChrono(inst) => inst.handle(msg, serv, kern),
            ServInst::EtcFSM(inst) => inst.handle(msg, serv, kern),
            ServInst::GFX2D(inst) => inst.handle(msg, serv, kern),
            ServInst::MathInt(inst) => inst.handle(msg, serv, kern),
            ServInst::SysTask(inst) => inst.handle(msg, serv, kern),
            ServInst::SysUsr(inst) => inst.handle(msg, serv, kern),
        }
    }
}

pub trait ServHlr: FromUnit {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr>;
    fn handle(&mut self, msg: Msg, serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr>;
}
