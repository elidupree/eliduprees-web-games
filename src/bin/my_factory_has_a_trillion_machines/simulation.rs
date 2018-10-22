use super::*;

use nalgebra::Vector2;
use arrayvec::ArrayVec;

type Length = i64;
const MAX_COMPONENTS: usize = 32;

enum SingularComponentType {
  Conveyor,
  Producer,
  Consumer,
}

enum ComponentType {
  Singular (SingularComponentType),
  Group (u16),
}

pub struct Component {
  position: Vector2 <Length>,
  scale: u8,
  facing: u8,
  component_type: ComponentType,
}

pub struct Group {
  size: Vector2 <Length>,
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
  average_color: [f64; 3],
}


pub struct Map {
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
}



fn step_component (component:) {
  match component.component_type {
    ComponentType::Singular (SingularComponentType::Conveyor) => {
      destination.push_material (mem::replace (&mut component.carried, None));
    },
    
  }
}



fn step (component_graph: ?????) {
  for component in component_graph.iterate_from_lasts() {
    component.step();
  }
}
