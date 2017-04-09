
extern crate num;

#[cfg(feature = "derive")]
extern crate serde;

pub mod vec;
pub mod mat;
#[cfg(feature = "derive")]
mod serialize;

pub use vec::*;
pub use mat::*;

