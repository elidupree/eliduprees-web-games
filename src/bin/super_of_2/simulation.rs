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
  pub velocity: Vector2 <f64>,
  
  pub inventory: Option <Inventory>,
}

#[derive (Copy, Clone, Debug)]
pub enum EntityPhysicalPosition {
  Map {center: Vector2 <f64>},
  Inventory {owner: Index, position: Vector2 <i32>,},
}
#[derive (Copy, Clone, Debug)]
pub enum EntityPosition {
  Physical (EntityPhysicalPosition),
  BeingDragged {physical: EntityPhysicalPosition, hovering_at: Vector2 <f64>},
}

#[derive (Debug)]
pub struct Inventory {
  pub size: Vector2 <i32>,
  pub slots: HashMap <Vector2 <i32>, Index>,
}

#[derive (PartialEq, Eq, Debug, Default)]
pub struct Tile {
  pub entities: Vec<Index>,
  pub terrain: Terrain,
}

#[derive (PartialEq, Eq, Debug, Derivative)]
#[derivative (Default)]
pub enum Terrain {
  #[derivative (Default)]
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
  pub dragged_entity: Option <Index>,
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


pub fn grid_location (location: Vector2 <f64>)->Vector2 <i32> {
  Vector2::new (location [0].floor() as i32, location [1].floor() as i32)
}
pub fn overlapping_tiles (center: Vector2 <f64>, size: Vector2 <f64>)->Vec<Vector2 <i32>> {
  let min = center - size/2.0;
  let max = center + size/2.0;
  (min [0].floor() as i32..max [0].ceil() as i32).flat_map(|x| {
    (min [1].floor() as i32..max [1].ceil() as i32).map(move |y| {
      Vector2::new (x,y)
    })
  }).collect()
}

impl State {
  pub fn create_entity (&mut self, entity: Entity)->Index {
    let index = self.next_index;
    self.next_index += 1;
    self.entities.insert (index, entity) ;
    self.insert_position_records (index);
    index
  }
  
  fn insert_position_records (&mut self, index: Index) {
    match self.entities [& index].position {
      EntityPosition::Physical (physical) => match physical {
        EntityPhysicalPosition::Map {center} => {
          for tile_location in overlapping_tiles (center, self.entities [& index].size) {
            let tile = self.map.entry (tile_location).or_insert (Default::default());
            if !tile.entities.contains (& index) {tile.entities.push (index);}
          }
        },
        EntityPhysicalPosition::Inventory {owner, position} => {
          if let Some(&mut Entity {inventory: Some(ref mut inventory), ..}) = self.entities.get_mut (& owner) {
            inventory.slots.insert (position, index);
          }
        },
      },
      EntityPosition::BeingDragged {..} => {
        assert!(self.dragged_entity == None);
        self.dragged_entity = Some (index);
      },
    }
  }
  fn remove_position_records (&mut self, index: Index) {
    match self.entities [& index].position {
      EntityPosition::Physical (physical) => match physical {
        EntityPhysicalPosition::Map {center} => {
          for tile_location in overlapping_tiles (center, self.entities [& index].size) {
            if {
              let tile = self.map.get_mut (& tile_location).expect ("position records should have existed for entity");
              tile.entities.retain (| &whatever | whatever != index);
              tile.entities.is_empty()
            } {
              self.map.remove (&tile_location) ;
            }
          }
        },
        EntityPhysicalPosition::Inventory {owner, position} => {
          if let Some(&mut Entity {inventory: Some(ref mut inventory), ..}) = self.entities.get_mut (& owner) {
            inventory.slots.insert (position, index);
          }
        },
      },
      EntityPosition::BeingDragged {..} => {
        assert!(self.dragged_entity == Some (index));
        self.dragged_entity = None;
      },
    }
  }
  pub fn move_entity (&mut self, index: Index, new_position: EntityPosition) {
    self.remove_position_records (index);
    {
      let entity = self.entities.get_mut (& index).unwrap();
      entity.position = new_position;
      match new_position {
        EntityPosition::Physical(EntityPhysicalPosition::Inventory {..}) => entity.velocity = Vector2::new (0.0, 0.0),
        _=>(),
      }
    }
    self.insert_position_records (index);
  }
  
  pub fn simulate (&mut self, duration: f64) {
    let tick_start = self.now;
    self.now += duration;
    let now = self.now;
    let constants = self.constants.clone();
    
    match self.pointer_state {
      PointerState::DragEntity {entity, current} => {
        let new_position = self.hover_to (self.entities [& entity].position, current);
        self.move_entity (entity, new_position);
      },
      _=>(),
    }
    
    let entities: Vec<_> = self.entities.keys().cloned().collect();
    for index in entities {
      match self.entities [& index].position {
        EntityPosition::Physical (EntityPhysicalPosition::Map {center}) => {
          let new_center = center + self.entities [& index].velocity*duration;
          self.move_entity (index, EntityPosition::Physical (EntityPhysicalPosition::Map {center: new_center}));
        },
        _=>(),
      }
    }
  }
  
  pub fn screen_to_physical (&self, location: Vector2 <f64>)->EntityPhysicalPosition {
    // TODO: inventories
    EntityPhysicalPosition::Map {center: location/self.map_scale}
  }
  pub fn hover_to (&self, original: EntityPosition, screen: Vector2 <f64>)->EntityPosition {
    // TODO: inventories
    let physical = match original {
      EntityPosition::Physical (physical) => physical,
      EntityPosition::BeingDragged {physical, ..} => physical,
    };
    let hover = match physical {
      EntityPhysicalPosition::Map {..} => screen/self.map_scale,
      _=> unimplemented!(),
    };
    EntityPosition::BeingDragged {physical: physical, hovering_at: hover}
  }
  
  pub fn position_to_screen (&self, position: EntityPosition)->Option<(Vector2 <f64>, f64)> {
    // TODO: inventories
    match position {
      EntityPosition::Physical (EntityPhysicalPosition::Map {center}) => {
        Some ((center*self.map_scale, self.map_scale))
      },
      EntityPosition::BeingDragged {physical: EntityPhysicalPosition::Map {..}, hovering_at} => {
        Some ((hovering_at*self.map_scale, self.map_scale))
      },
      EntityPosition::Physical (EntityPhysicalPosition::Inventory {owner:_, position:_}) => {
        None
      },
      EntityPosition::BeingDragged {physical: EntityPhysicalPosition::Inventory {owner:_, ..}, hovering_at:_} => {
        None
      },
    }
  }
  
  pub fn entity_overlaps_screen_location (&self, index: Index, location: Vector2 <f64>)->bool {
    let entity = & self.entities [& index];
    if let Some((position, scale)) = self.position_to_screen (entity.position) {
      let half_size = entity.size*scale/2.0;
      location[0] >= position[0] - half_size[0] &&
      location[0] <= position[0] + half_size[0] &&
      location[1] >= position[1] - half_size[1] &&
      location[1] <= position[1] + half_size[1]
    }
    else {false}
  }
  pub fn entity_at_screen_location (&self, location: Vector2 <f64>)->Option <Index> {
    match self.screen_to_physical (location) {
      EntityPhysicalPosition::Map {center} => {
        self.map.get (& grid_location (center)).and_then (| tile | tile.entities.iter().filter (| &&index | self.entity_overlaps_screen_location (index, location)).min().cloned())
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
      PointerState::PossibleClick {start:_, entity:_} => {
      
      },
      PointerState::DragEntity {entity, current} => {
        let new_position = EntityPosition::Physical(self.screen_to_physical (current));
        self.move_entity (entity, new_position);
      },
      PointerState::DragSelect {start:_, current:_} => {
      
      },
    }
    self.pointer_state = PointerState::Nowhere;
  }
}
