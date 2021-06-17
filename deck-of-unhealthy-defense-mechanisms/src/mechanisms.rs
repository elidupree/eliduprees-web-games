use crate::actions::{Action, RotateMechanism};
use crate::game::UPDATE_DURATION;
use crate::map::{
  Facing, FloatingVectorExtension, GridVector, GridVectorExtension, Map, Material, Rotation, Tile,
  TILE_RADIUS, TILE_SIZE, TILE_WIDTH,
};
use crate::ui_glue::Draw;
use eliduprees_web_games_lib::auto_constant;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

pub struct MechanismUpdateContext<'a> {
  pub position: GridVector,
  pub map: &'a mut Map,
  pub former: &'a Map,
}

pub struct MechanismImmutableContext<'a> {
  pub position: GridVector,
  pub map: &'a Map,
}

impl<'a> MechanismUpdateContext<'a> {
  pub fn this_tile(&self) -> &Tile {
    self.map.tiles.get(&self.position).unwrap()
  }
  pub fn this_tile_mut(&mut self) -> &mut Tile {
    self.map.tiles.get_mut(&self.position).unwrap()
  }
  pub fn this_mechanism(&self) -> &Mechanism {
    self.this_tile().mechanism.as_ref().unwrap()
  }
  pub fn this_mechanism_mut(&mut self) -> &mut Mechanism {
    self.this_tile_mut().mechanism.as_mut().unwrap()
  }
  pub fn this_mechanism_type_mut<'b, T: TryFrom<&'b mut MechanismType>>(&'b mut self) -> T
  where
    T::Error: Debug,
  {
    (&mut self.this_mechanism_mut().mechanism_type)
      .try_into()
      .unwrap()
  }
}

impl<'a> MechanismImmutableContext<'a> {
  pub fn this_tile(&self) -> &Tile {
    self.map.tiles.get(&self.position).unwrap()
  }
  pub fn this_mechanism(&self) -> &Mechanism {
    self.this_tile().mechanism.as_ref().unwrap()
  }
}

#[allow(unused)]
pub trait MechanismTrait {
  /** Perform a single time-step update on this mechanism, possibly modifying the game state.

  Note that when called, `self` is a *copy* of the actual mechanism type; to modify the mechanism, you need to use `context`.
  */
  fn update(&self, context: MechanismUpdateContext);

  fn interactions(&self, context: MechanismImmutableContext) -> [Option<Action>; 2] {
    [None, None]
  }

  fn draw(&self, context: MechanismImmutableContext, draw: &mut impl Draw);
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Mechanism {
  pub mechanism_type: MechanismType,
  pub facing: Facing,
}

macro_rules! mechanism_enum {
  ($($Variant: ident,)*) => {
    #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
    pub enum MechanismType {
      $($Variant($Variant),)*
    }

    impl MechanismTrait for MechanismType {
      fn update(&self, context: MechanismUpdateContext) {
        match self {
          $(MechanismType::$Variant(s) => s.update(context),)*
        }
      }

      fn interactions(&self, context: MechanismImmutableContext) -> [Option<Action>; 2] {
        match self {
          $(MechanismType::$Variant(s) => s.interactions(context),)*
        }
      }

      fn draw(&self, context: MechanismImmutableContext, draw: &mut impl Draw) {
        match self {
          $(MechanismType::$Variant(s) => s.draw(context, draw),)*
        }
      }
    }

    $(
    impl<'a> TryFrom<&'a MechanismType> for &'a $Variant {
      type Error = ();
      fn try_from(value: &'a MechanismType) -> Result<&'a $Variant, Self::Error> {
        if let MechanismType::$Variant(s) = value {
          Ok(s)
        }
        else {
          Err(())
        }
      }
    }

    impl<'a> TryFrom<&'a mut MechanismType> for &'a mut $Variant {
      type Error = ();
      fn try_from(value: &'a mut MechanismType) -> Result<&'a mut $Variant, Self::Error> {
        if let MechanismType::$Variant(s) = value {
          Ok(s)
        }
        else {
          Err(())
        }
      }
    }
    )*
  }

}

mechanism_enum! {
  Deck,
  Conveyor,
}

impl Default for MechanismType {
  fn default() -> Self {
    MechanismType::Deck(Deck {})
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Deck {}

impl MechanismTrait for Deck {
  fn update(&self, mut context: MechanismUpdateContext) {
    for facing in Facing::ALL_FACINGS {
      let target_tile_position = context.position + facing.unit_vector() * TILE_WIDTH;
      let target = context.position.to_floating()
        + facing.unit_vector().to_floating() * (TILE_RADIUS as f64 * 1.01);
      if let Some(old_target_tile) = context.former.tiles.get(&target_tile_position) {
        if old_target_tile
          .materials
          .iter()
          .all(|m| (m.position - target).magnitude() > TILE_WIDTH as f64)
        {
          let tile = context
            .this_tile_mut()
            .materials
            .push(Material { position: target });
        }
      }
    }
  }

  fn interactions(&self, context: MechanismImmutableContext) -> [Option<Action>; 2] {
    [None, None]
  }

  fn draw(&self, context: MechanismImmutableContext, draw: &mut impl Draw) {
    draw.rectangle_on_map(
      10,
      context.position.to_floating(),
      TILE_SIZE.to_floating(),
      "#f66",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Conveyor {}

impl MechanismTrait for Conveyor {
  fn update(&self, mut context: MechanismUpdateContext) {
    let mechanism = context.this_mechanism();
    let target_tile_position = context.position + mechanism.facing.unit_vector() * TILE_WIDTH;
    let target = context.position.to_floating()
      + mechanism.facing.unit_vector().to_floating() * (TILE_RADIUS as f64 * 1.01);
    let former = context.former;
    let tile = context.this_tile_mut();
    if let Some(material) = tile
      .materials
      .iter_mut()
      .min_by_key(|m| OrderedFloat((m.position - target).magnitude()))
    {
      if let Some(old_target_tile) = former.tiles.get(&target_tile_position) {
        if old_target_tile
          .materials
          .iter()
          .all(|m| (m.position - material.position).magnitude() > TILE_WIDTH as f64)
        {
          material.position.move_towards(
            target,
            auto_constant("conveyor_speed", 2.3) * UPDATE_DURATION,
          )
        }
      }
    }
  }

  fn interactions(&self, context: MechanismImmutableContext) -> [Option<Action>; 2] {
    [
      Some(Action::RotateMechanism(RotateMechanism::new(
        Rotation::COUNTERCLOCKWISE,
      ))),
      Some(Action::RotateMechanism(RotateMechanism::new(
        Rotation::CLOCKWISE,
      ))),
    ]
  }

  fn draw(&self, context: MechanismImmutableContext, draw: &mut impl Draw) {
    let center = context.position.to_floating();
    let facing = context.this_mechanism().facing;
    let forwards = facing.unit_vector().to_floating();
    draw.rectangle_on_map(10, center, TILE_SIZE.to_floating(), "#888");
    draw.rectangle_on_map(
      10,
      center + forwards * (TILE_RADIUS as f64 * 0.6),
      TILE_SIZE.to_floating() * 0.15,
      "#bbb",
    );
    for rotation in [Rotation::CLOCKWISE, Rotation::COUNTERCLOCKWISE] {
      draw.rectangle_on_map(
        10,
        center
          + forwards * (TILE_RADIUS as f64 * 0.3)
          + (facing + rotation).unit_vector().to_floating() * (TILE_RADIUS as f64 * 0.3),
        TILE_SIZE.to_floating() * 0.15,
        "#bbb",
      );
    }
  }
}
