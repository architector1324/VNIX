#![feature(array_chunks)]
#![feature(extract_if)]
#![feature(iter_array_chunks)]
#![feature(type_alias_impl_trait)]
#![feature(iterator_try_reduce)]
#![feature(iterator_try_collect)]
#![feature(associated_type_defaults)]
#![feature(slice_as_chunks)]
#![feature(async_closure)]

extern crate alloc;

pub mod vnix;
pub mod driver;
pub mod content;
