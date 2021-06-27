use crate::actions::{Action, Reshuffle, SimpleAction, SimpleActionType};
use crate::game::{Game, Time};
use crate::geometry::{
  Facing, FloatingVectorExtension, GridVector, GridVectorExtension, Rotation, TILE_RADIUS,
  TILE_SIZE, TILE_WIDTH,
};
use crate::movers::{Material, Mover, MoverBehavior, MoverType, Projectile};
use crate::ui_glue::Draw;
use eliduprees_web_games_lib::auto_constant;
use ordered_float::OrderedFloat;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Mechanism {
  pub mechanism_type: MechanismType,
}

#[derive(Debug)]
pub struct TypedMechanismView<'a, T> {
  mechanism: &'a mut Mechanism,
  _marker: PhantomData<T>,
}

impl<'a, T> TypedMechanismView<'a, T> {
  fn mechanism_type<'b>(&'b self) -> &'b T
  where
    &'b T: TryFrom<&'b MechanismType>,
  {
    match (&self.mechanism.mechanism_type).try_into() {
      Ok(b) => b,
      Err(_) => panic!("A TypedMechanismView existed for the wrong type"),
    }
  }
  fn mechanism_type_mut<'b>(&'b mut self) -> &'b mut T
  where
    &'b mut T: TryFrom<&'b mut MechanismType>,
  {
    match (&mut self.mechanism.mechanism_type).try_into() {
      Ok(b) => b,
      Err(_) => panic!("A TypedMechanismView existed for the wrong type"),
    }
  }
}

impl<'a, T> Deref for TypedMechanismView<'a, T> {
  type Target = Mechanism;

  fn deref(&self) -> &Mechanism {
    self.mechanism
  }
}

impl<'a, T> DerefMut for TypedMechanismView<'a, T> {
  fn deref_mut(&mut self) -> &mut Mechanism {
    &mut self.mechanism
  }
}

pub struct MechanismUpdateContext<'a> {
  pub position: GridVector,
  pub game: &'a mut Game,
}

pub struct MechanismImmutableContext<'a> {
  pub position: GridVector,
  pub game: &'a Game,
}

impl<'a> MechanismUpdateContext<'a> {
  pub fn this(&self) -> &Mechanism {
    self.game.mechanism(self.position).unwrap()
  }
  pub fn mutate_this<T, R, F: FnOnce(TypedMechanismView<T>) -> R>(&mut self, f: F) -> R {
    self
      .game
      .mutate_mechanism(self.position, |mechanism| {
        f(TypedMechanismView {
          mechanism,
          _marker: PhantomData,
        })
      })
      .unwrap()
  }
}

impl<'a> MechanismImmutableContext<'a> {
  pub fn this(&self) -> &Mechanism {
    self.game.mechanism(self.position).unwrap()
  }
}

#[allow(unused)]
pub trait MechanismTrait {
  /** Perform a single scheduled update on this mechanism, possibly modifying the game state.

  Note that when called, `self` is a *copy* of the actual mechanism type; to modify the mechanism, you need to use `context`.
  */
  fn wake(&self, context: MechanismUpdateContext) {}
  fn next_wake(&self, this: &Mechanism) -> Option<Time> {
    None
  }

  fn activation(&self, context: MechanismImmutableContext) -> Option<Action> {
    None
  }

  fn can_be_material_source(&self, facing: Facing) -> bool {
    false
  }

  fn wants_to_steal(&self, material: &Material) -> bool {
    false
  }

  fn draw(&self, context: MechanismImmutableContext, draw: &mut dyn Draw);
}

trait_enum! {
  #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
  pub enum MechanismType: MechanismTrait {
    Deck,
    Conveyor,
    Tower,
  }
}

pub trait BuildMechanismTrait {
  fn mechanism(&self, game: &Game) -> Mechanism;
}

trait_enum! {
  #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
  pub enum BuildMechanism: BuildMechanismTrait {
    BuildTower,
  }
}

impl Default for MechanismType {
  fn default() -> Self {
    MechanismType::Deck(Deck {})
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Deck {}

impl MechanismTrait for Deck {
  fn wake(&self, context: MechanismUpdateContext) {
    // for facing in Facing::ALL_FACINGS {
    //   let target_tile_position = context.position + facing.unit_vector() * TILE_WIDTH;
    //   let target = context.position.to_floating()
    //     + facing.unit_vector().to_floating() * (TILE_RADIUS as f64 * 1.01);
    //   if let Some(old_target_tile) = context.game.grid.get(target_tile_position) {
    //     if old_target_tile
    //       .movers
    //       .iter()
    //       .map(|&id| context.game.mover(id).unwrap())
    //       .all(|m| (m.position - target).magnitude() > TILE_WIDTH as f64)
    //     {
    //       context.game.create_mover(Mover {
    //         position: target,
    //         mover_type: MoverType::Material,
    //         behavior: MoverBehavior::Material(Material),
    //         ..Default::default()
    //       });
    //     }
    //   }
    // }
  }

  fn activation(&self, _context: MechanismImmutableContext) -> Option<Action> {
    Some(Action::SimpleAction(SimpleAction::new(
      5,
      Some(50),
      "Reshuffle",
      "",
      "",
      false,
      SimpleActionType::Reshuffle(Reshuffle),
    )))
  }

  fn can_be_material_source(&self, _facing: Facing) -> bool {
    true
  }

  fn draw(&self, context: MechanismImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.position.to_floating(),
      TILE_SIZE.to_floating() * 0.9,
      "#f66",
    );
  }
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum ConveyorSide {
  Input,
  Output,
  Disconnected,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Conveyor {
  pub sides: [ConveyorSide; 4],
  pub last_sent: Facing,
}

impl MechanismTrait for Conveyor {
  fn wake(&self, mut context: MechanismUpdateContext) {
    // let sides = self.sides;
    // let mut facings = Facing::ALL_FACINGS;
    // facings.rotate_left(self.last_sent.as_index() + 1);
    // // todo: handle multiple outputs reasonably
    // for &facing in &facings {
    //   // todo: not so messy code
    //   if !(sides[facing.as_index()] == ConveyorSide::Output
    //     || context
    //       .game
    //       .grid
    //       .get(context.position + facing.unit_vector() * TILE_WIDTH)
    //       .map_or(false, |tile| {
    //         tile.mechanism.as_ref().map_or(false, |mechanism| {
    //           mechanism.mechanism_type.wants_to_steal(&Material)
    //         })
    //       }))
    //   {
    //     continue;
    //   }
    //   let target_tile_position = context.position + facing.unit_vector() * TILE_WIDTH;
    //   let target = context.position.to_floating()
    //     + facing.unit_vector().to_floating() * (TILE_RADIUS as f64 * 1.01);
    //   let tile = context.this_tile();
    //   if let Some(&material_id) = tile.movers.iter().min_by_key(|&&id| {
    //     let m = context.game.mover(id).unwrap();
    //     OrderedFloat((m.position - target).magnitude())
    //   }) {
    //     let material_position = context.game.mover(material_id).unwrap().position;
    //     if let Some(old_target_tile) = former.grid.get(target_tile_position) {
    //       if old_target_tile
    //         .movers
    //         .iter()
    //         .map(|&id| context.game.mover(id).unwrap())
    //         .filter(|mover| mover.mover_type == MoverType::Material)
    //         .all(|m| (m.position - material_position).magnitude() > TILE_WIDTH as f64)
    //       {
    //         let conveyor_position = context.position;
    //         let escaped = context
    //           .game
    //           .mutate_mover(material_id, |material| {
    //             material.position.move_towards(
    //               target,
    //               auto_constant("conveyor_speed", 2.3) * UPDATE_DURATION,
    //             );
    //             material.position.containing_tile() != conveyor_position
    //           })
    //           .unwrap();
    //         if escaped {
    //           context.this_mechanism_type_mut::<Self>().last_sent = facing;
    //         }
    //         break;
    //       }
    //     }
    //   }
    // }
  }

  fn can_be_material_source(&self, facing: Facing) -> bool {
    self.sides[facing.as_index()] != ConveyorSide::Input
  }

  fn draw(&self, context: MechanismImmutableContext, draw: &mut dyn Draw) {
    let center = context.position.to_floating();
    draw.rectangle_on_map(10, center, TILE_SIZE.to_floating() * 0.9, "#888");
    for (&facing, side) in Facing::ALL_FACINGS.iter().zip(self.sides) {
      if side == ConveyorSide::Output {
        let forwards = facing.unit_vector().to_floating();
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
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Tower {
  pub volition_base_time: Time,
  pub volition_at_base_time: f64,
  pub maximum_volition: f64,
  pub range: f64,
  pub next_wake: Time,
}

const TOWER_WAKE_DELAY: Time = 0.1;

impl Tower {
  pub fn volition(&self, time: Time) -> f64 {
    (self.volition_at_base_time
      + auto_constant("tower_regeneration", 1.0) * (time - self.volition_base_time))
      .min(self.maximum_volition)
  }
  pub fn rebase(&mut self, time: Time) {
    self.volition_at_base_time = self.volition(time);
    self.volition_base_time = time;
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct BuildTower;
impl BuildMechanismTrait for BuildTower {
  fn mechanism(&self, game: &Game) -> Mechanism {
    Mechanism {
      mechanism_type: MechanismType::Tower(Tower {
        volition_base_time: game.physics_time,
        volition_at_base_time: 0.0,
        maximum_volition: 5.0,
        range: 5.0 * TILE_WIDTH as f64,
        next_wake: game.physics_time,
      }),
    }
  }
}

impl MechanismTrait for Tower {
  fn wake(&self, mut context: MechanismUpdateContext) {
    let position = context.position.to_floating();
    let mut volition = self.volition(context.game.physics_time);
    // for id in context.this_tile().movers.clone() {
    //   let mover = context.game.mover(id).unwrap();
    //   if mover.mover_type == MoverType::Material {
    //     context.game.remove_mover(id);
    //     context.this_mechanism_type_mut::<Self>().volition += 1.0;
    //   }
    // }

    if volition >= self.maximum_volition {
      if let Some((_, target)) = context
        .game
        .movers_near(context.position.to_floating(), self.range)
        .filter(|&(_id, mover)| mover.mover_type == MoverType::Monster)
        .min_by_key(|&(_id, mover)| {
          OrderedFloat((mover.position(context.game.physics_time) - position).magnitude_squared())
        })
      {
        let difference = target.position(context.game.physics_time) - position;
        let speed = auto_constant("shot_speed", 10.0);
        context.game.create_mover(Mover {
          trajectory_base_time: context.game.physics_time,
          position_at_base_time: position,
          velocity: difference * (speed / difference.magnitude()),
          mover_type: MoverType::Projectile,
          behavior: MoverBehavior::Projectile(Projectile {
            disappear_time: context.game.physics_time + self.range / speed,
          }),
          ..Default::default()
        });
        volition -= 5.0;
      }
    }

    let now = context.game.physics_time;
    context.mutate_this(|mut this: TypedMechanismView<Self>| {
      let this = this.mechanism_type_mut();
      this.rebase(now);
      this.volition_at_base_time = volition.min(self.maximum_volition);
      this.next_wake = now + TOWER_WAKE_DELAY * rand::thread_rng().gen_range(0.95..1.05);
    });
  }

  fn next_wake(&self, _this: &Mechanism) -> Option<Time> {
    Some(self.next_wake)
  }

  fn wants_to_steal(&self, _material: &Material) -> bool {
    true //self.volition() < self.maximum_volition
  }

  fn draw(&self, context: MechanismImmutableContext, draw: &mut dyn Draw) {
    let center = context.position.to_floating();
    let brightness =
      ((0.5 + 0.5 * (self.volition(context.game.physics_time) / self.maximum_volition)) * 255.0)
        .round();
    draw.rectangle_on_map(
      10,
      center,
      TILE_SIZE.to_floating(),
      &format!("rgb({}, {}, {})", brightness, brightness, brightness),
    );
  }
}
