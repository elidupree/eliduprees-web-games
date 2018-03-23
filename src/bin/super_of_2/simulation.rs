use super::*;

use rand::Rng;
use boolinator::Boolinator;
use nalgebra::Vector2;
use std::collections::{HashMap, BTreeMap};

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
  BeingDragged {physical: EntityPhysicalPosition, hovering_at: Vector2 <f64>},
}

#[derive (Debug)]
pub struct Inventory {
  pub size: Vector2 <i32>,
  pub slots: HashMap <Vector2 <i32>, Index>,
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

#[derive (Debug)]
pub enum PointerState {
  Nowhere,
  //Hovering (Vector2 <f64>),
  PossibleClick {start: Vector2 <f64>, entity: Option <Index>},
  DragEntity {entity: Index, current: Vector2 <f64>},
  DragSelect {start: Vector2 <f64>, current: Vector2 <f64>},
}

#[derive (Derivative)]
#[derivative (Default)]
pub struct State {
  pub entities: BTreeMap <Index, Entity>,
  pub next_index: Index,
  pub map: HashMap <Vector2 <i32>, Tile>,
  #[derivative (Default (value = "PointerState::Nowhere"))]
  pub pointer_state: PointerState,
  
  #[derivative (Default (value = "16.0"))]
  pub map_scale: f64,
  #[derivative (Default (value = "Vector2::new (0.0, 0.0)"))]
  pub map_offset: Vector2 <f64>,
  
  #[derivative (Default (value = "Box::new(::rand::ChaChaRng::new_unseeded())"))]
  pub generator: Box <Rng>,
  pub constants: Rc<Constants>,
  pub now: f64,
}


impl Entity {
}

pub fn grid_location (location: Vector2 <f64>)->Vector2 <i32> {
  Vector2::new (location [0].trunc() as i32, location [1].trunc() as i32)
}

impl State {
  pub fn create_entity (&mut self, entity: Entity)->Index {
    let index = self.next_index;
    self.next_index += 1;
    self.entities.insert (index, entity) ;
    index
  }
  pub fn simulate (&mut self, duration: f64) {
    let tick_start = self.now;
    self.now += duration;
    let now = self.now;
    let constants = self.constants.clone();
    
    for entity in self.entities.iter_mut() {
      match entity.position {
        EntityPosition::Physical (EntityPhysicalPosition {ref mut center, velocity}) => {
          *center += velocity*duration;
        },
        _=>(),
      }
    }
  }
  
  pub fn screen_to_physical (&self, location: Vector2 <f64>)->EntityPhysicalPosition {
    // TODO: inventories
    EntityPhysicalPosition::Map {center: location/self.map_scale, velocity: Vector2::new (0.0, 0.0)}
  }
  
  pub fn physical_to_screen (&self, location: EntityPhysicalPosition)->Option<Vector2 <f64>> {
    // TODO: inventories
    match location {
      EntityPhysicalPosition::Map {center, ..} => {
        Some (center*self.map_scale)
      },
      EntityPhysicalPosition::Inventory {owner, position} => {
        None
      },
    }
  }
  
  
  pub fn entity_at_screen_location (&self, location: Vector2 <f64>)->Option <Index> {
    match self.screen_to_physical (location) {
      EntityPhysicalPosition::Map {center, ..} => {
        self.map.get (& grid_location (center)).and_then (| tile | tile.entities.iter().min().cloned())
      },
      EntityPhysicalPosition::Inventory {owner, position} => {
        self.entities.get (& owner).and_then (
          | entity | entity.inventory.as_ref().and_then (
            | inventory | inventory.slots.get (&position).cloned()
          )
        )
      },
    }
  }
  
  pub fn cancel_gesture(&mut self) {
    self.pointer_state = PointerState::Nowhere;
  }
  pub fn finish_gesture(&mut self) {
    match self.pointer_state {
      PointerState::Nowhere => (),
      PointerState::PossibleClick {start, entity} => {
      
      },
      PointerState::DragEntity {entity, current} => {
        
      },
      PointerState::DragSelect {start, current} => {
      
      },
    }
    self.pointer_state = PointerState::Nowhere;
  }
}
