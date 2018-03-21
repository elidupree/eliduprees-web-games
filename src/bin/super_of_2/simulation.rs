use super::*;

use rand::Rng;
use boolinator::Boolinator;
use nalgebra::Vector2;
use std::collections::HashMap;

use std::rc::Rc;
use std::cell::Cell;
use std::str::FromStr;


pub type Index = u64;

#[derive (Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub struct Constants {
}
js_serializable! (Constants);
js_deserializable! (Constants);

pub type InventorySlot = Option <Index>;

#[derive (Debug)]
pub struct Entity {
  pub is_object: bool,
  pub is_unit: bool,
  
  // TODO: hearts, experience, gold, cost,
  
  pub size: Vector2 <f64>,
  pub position: EntityPosition,
  
  pub inventory: Option <Inventory>,
}

#[derive (Debug)]
pub enum EntityPhysicalPosition {
  Map {center: Vector2 <f64>, velocity: Vector2 <f64>,},
  Inventory {owner: Index, position: Vector2 <i32>,},
}
#[derive (Debug)]
pub enum EntityPosition {
  Physical (EntityPhysicalPosition),
  BeingDragged (EntityPhysicalPosition, Vector2 <f64>),
}

#[derive (Debug)]
pub struct Inventory {
  pub size: Vector2 <i32>,
  pub slots: HashMap <Vector2 <i32>, InventorySlot>,
}

#[derive (Debug)]
pub struct Tile {
  pub entities: Vec<Index>,
  pub terrain: Terrain,
}

#[derive (Debug)]
pub enum Terrain {
  Nothing,
  Wall,
}

#[derive (Derivative)]
#[derivative (Default)]
pub struct State {
  pub entities: HashMap <Index, Entity>,
  pub map: HashMap <Vector2 <i32>, Tile>,
  
  
  #[derivative (Default (value = "Box::new(::rand::ChaChaRng::new_unseeded())"))]
  pub generator: Box <Rng>,
  pub constants: Rc<Constants>,
  pub now: f64,
}


impl Entity {
}

impl State {
  pub fn simulate (&mut self, duration: f64) {
    let tick_start = self.now;
    self.now += duration;
    let now = self.now;
    let constants = self.constants.clone();
  }
}
