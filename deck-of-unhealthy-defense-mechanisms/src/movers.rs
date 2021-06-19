use crate::game::{Game, UPDATE_DURATION};
use crate::map::{
  FloatingVector, FloatingVectorExtension, GridVectorExtension, Map, TILE_SIZE, TILE_WIDTH,
};
use crate::ui_glue::Draw;
use derivative::Derivative;
use eliduprees_web_games_lib::auto_constant;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use trait_enum::trait_enum;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default)]
pub enum MoverType {
  #[derivative(Default)]
  Monster,
  Projectile,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Mover {
  pub position: FloatingVector,
  pub velocity: FloatingVector,
  pub home: FloatingVector,
  pub active_time: Range<f64>,
  pub mover_type: MoverType,
  pub hitpoints: f64,
  pub behavior: MoverBehavior,
}

pub struct MoverUpdateContext<'a> {
  pub this: &'a mut Mover,
  pub map: &'a mut Map,
  pub former_game: &'a Game,
  pub destroyed: bool,
}

pub struct MoverImmutableContext<'a> {
  pub this: &'a Mover,
  pub game: &'a Game,
}

pub trait MoverBehaviorTrait {
  /** Perform a single time-step update on this mover, possibly modifying the game state.

  Note that when called, `self` is a *copy* of the actual MoverBehavior implementor;
  `context.this` is a copy of the actual Mover, and will overwrite the original after update() returns.
  MoverBehavior implementors are expected to have no data; they use the shared data that is in Mover.
  It is an error for update() to remove any existing Movers, but it may mutate them, and it may push
  new Movers into tiles (which will not get an update this tick).
  */
  fn update(&self, context: &mut MoverUpdateContext);

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw);
}

trait_enum! {
  #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
  pub enum MoverBehavior: MoverBehaviorTrait {
    Monster, Projectile
  }
}

impl Default for MoverBehavior {
  fn default() -> Self {
    MoverBehavior::Monster(Monster)
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Monster;

impl MoverBehaviorTrait for Monster {
  fn update(&self, context: &mut MoverUpdateContext) {
    let active = context
      .this
      .active_time
      .contains(&context.former_game.day_progress);
    let target;
    if active {
      target = FloatingVector::zeros();
    } else {
      target = context.this.home;
    }

    let relative_target = target - context.this.position;

    let acceleration = auto_constant("monster_acceleration", 4.0) * TILE_WIDTH as f64;
    let max_speed = auto_constant("monster_max_speed", 1.6) * TILE_WIDTH as f64;
    // Account for stopping distance:
    let target_speed = max_speed.min((2.0 * relative_target.magnitude() * acceleration).sqrt());
    let target_velocity = relative_target.normalize() * target_speed;
    context
      .this
      .velocity
      .move_towards(target_velocity, acceleration * UPDATE_DURATION);
  }

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.this.position,
      TILE_SIZE.to_floating() * 0.8,
      "#000",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Projectile;

impl MoverBehaviorTrait for Projectile {
  fn update(&self, context: &mut MoverUpdateContext) {
    context.this.hitpoints -= UPDATE_DURATION;
    if context.this.hitpoints <= 0.0 {
      context.destroyed = true;
    }
  }

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.this.position,
      TILE_SIZE.to_floating() * 0.2,
      "#fff",
    );
  }
}
