#![feature(min_type_alias_impl_trait)]
#![feature(map_first_last)]

#[allow(unused_macros)]
macro_rules! debug {
  ($($args:tt)*) => {
    $crate::ui_glue::js::debug(&format!($($args)*));
  }
}

pub mod actions;
pub mod cards;
pub mod game;
pub mod geometry;
pub mod mechanisms;
pub mod movers;
pub mod ui_glue;

//use misc;
//use modules::{self, Module};
