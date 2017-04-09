
extern crate num;

#[cfg(feature = "derive")]
extern crate serde;

pub mod vec;
pub mod mat;
#[cfg(feature = "derive")]
mod derive;

pub use vec::*;
pub use mat::*;

