use super::*;

use stdweb::unstable::TryInto;
use ordered_float::OrderedFloat;
use std::collections::BTreeSet;


impl State {
  pub fn draw (&self) {
    // TODO: restrict to on-screen tiles
    let drawn_entities = BTreeSet::new();
    for (location, tile) in self.map.iter() {
      drawn_entities.extend (tile.entities.iter().cloned());
      
      /*js!{
        canvas.strokeRect (
      }*/
    }
    for index in drawn_entities {
      let entity = & self.entities [&index];
      let (position, scale) = self.physical_to_screen (entity.position).expect ("entities without screen-relative positions shouldn't have been considered by the draw code");
      let size = entity.size*scale;
      let corner = position - size/2.0;
      js!{
        context.strokeStyle = "rgb(0, 0, 0)";
        context.lineWidth = 1;
        canvas.strokeRect (@{corner [0]}, @{corner [1]}, @{size [0]}, @{size [1]});
      }
    }
  }
}
