
extern crate num;

#[cfg(feature = "serialize")]
extern crate serde;

pub mod vec;
pub mod mat;
#[cfg(feature = "serialize")]
mod serialize;

pub use vec::*;
pub use mat::*;

