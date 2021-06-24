use crate::game::{Game, UPDATE_DURATION};
use crate::map::{
  FloatingVector, FloatingVectorExtension, GridVectorExtension, Map, TILE_RADIUS, TILE_SIZE,
  TILE_WIDTH,
};
use crate::ui_glue::Draw;
use derivative::Derivative;
use eliduprees_web_games_lib::auto_constant;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use trait_enum::trait_enum;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default)]
pub enum MoverType {
  #[derivative(Default)]
  Monster,
  Projectile,
  Material,
}
#[derive(
  Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug, Default,
)]
pub struct MoverId(pub usize);

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
  pub id: MoverId,
  pub map: &'a mut Map,
  pub former_game: &'a Game,
}

pub struct MoverImmutableContext<'a> {
  pub id: MoverId,
  pub game: &'a Game,
}

impl<'a> MoverUpdateContext<'a> {
  pub fn this(&self) -> &Mover {
    self.map.movers.get(self.id).unwrap()
  }
  pub fn mutate_this<R, F: FnOnce(&mut Mover) -> R>(&mut self, f: F) -> R {
    self.map.mutate_mover(self.id, f).unwrap()
  }
  // take self by value unnecessarily, to protect from accidentally doing stuff after destroyed
  pub fn destroy_this(self) {
    self.map.remove_mover(self.id);
  }
}

impl<'a> MoverImmutableContext<'a> {
  pub fn this(&self) -> &Mover {
    self.game.map.movers.get(self.id).unwrap()
  }
}

#[allow(unused)]
pub trait MoverBehaviorTrait {
  /** Perform a single time-step update on this mover, possibly modifying the game state.

  Note that when called, `self` is a *copy* of the actual MoverBehavior implementor;
  use `context.mutate_this()` to change this Mover.
  MoverBehavior implementors are expected to have no data; they use the shared data that is in Mover.
  */
  fn update(&self, context: MoverUpdateContext) {}

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw);
}

trait_enum! {
  #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
  pub enum MoverBehavior: MoverBehaviorTrait {
    Monster, Projectile, Material,
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
  fn update(&self, mut context: MoverUpdateContext) {
    let active = context
      .this()
      .active_time
      .contains(&context.former_game.day_progress);
    let target;
    if active {
      target = FloatingVector::zeros();
    } else {
      target = context.this().home;
    }

    let relative_target = target - context.this().position;

    let acceleration = auto_constant("monster_acceleration", 4.0) * TILE_WIDTH as f64;
    let max_speed = auto_constant("monster_max_speed", 1.6) * TILE_WIDTH as f64;
    // Account for stopping distance:
    let target_speed = max_speed.min((2.0 * relative_target.magnitude() * acceleration).sqrt());
    let target_velocity = relative_target.normalize() * target_speed;
    context.mutate_this(|this| {
      this
        .velocity
        .move_towards(target_velocity, acceleration * UPDATE_DURATION)
    });
  }

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.this().position,
      TILE_SIZE.to_floating() * 0.8,
      "#000",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Projectile;

impl MoverBehaviorTrait for Projectile {
  fn update(&self, mut context: MoverUpdateContext) {
    context.mutate_this(|this| {
      this.hitpoints -= UPDATE_DURATION;
    });
    if context.this().hitpoints <= 0.0 {
      context.destroy_this();
      return;
    }
    if let Some((target_id, _)) = context
      .map
      .movers_near(context.this().position, 0.8 * TILE_RADIUS as f64)
      .filter(|&(_id, mover)| mover.mover_type == MoverType::Monster)
      .min_by_key(|&(_id, mover)| {
        OrderedFloat((mover.position - context.this().position).magnitude_squared())
      })
    {
      let impact = auto_constant("projectile_impact", 2.0) * TILE_WIDTH as f64;

      // Push the monster directly away from your deck. We COULD have the monster be propelled in the direction of the projectile (context.this().velocity) instead, but that causes the monster to be knocked around in a way that feels slippery, rather than feeling like a struggle of determination against determination. Having projectile directions matter would make this a physics game, and this I actually DON'T want this to be a physics game.
      context
        .map
        .mutate_mover(target_id, |target| {
          let direction = target.position;
          target.velocity += direction.normalize() * impact;
        })
        .unwrap();
      context.destroy_this();
    }
  }

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.this().position,
      TILE_SIZE.to_floating() * 0.2,
      "#fff",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Material;

impl MoverBehaviorTrait for Material {
  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      20,
      context.this().position,
      TILE_SIZE.to_floating() * 0.25,
      "#fff",
    );
  }
}
