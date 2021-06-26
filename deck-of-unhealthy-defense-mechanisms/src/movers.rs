use crate::game::{Game, Time};
use crate::geometry::GridBounds;
use crate::geometry::{
  FloatingVector, FloatingVectorExtension, GridVectorExtension, TILE_SIZE, TILE_WIDTH,
};
use crate::ui_glue::Draw;
use derivative::Derivative;
use eliduprees_web_games_lib::auto_constant;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Range};

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
  pub trajectory_base_time: Time,
  pub position_at_base_time: FloatingVector,
  pub velocity: FloatingVector,
  pub radius: f64,
  pub behavior: MoverBehavior,

  pub mover_type: MoverType,
}

impl Mover {
  pub fn position(&self, time: Time) -> FloatingVector {
    self.position_at_base_time + self.velocity * (time - self.trajectory_base_time)
  }

  pub fn bounds(&self, time: Time) -> [FloatingVector; 2] {
    let position = self.position(time);
    let half_size = FloatingVector::new(self.radius, self.radius);
    [position - half_size, position + half_size]
  }

  pub fn grid_bounds(&self, time: Time) -> GridBounds {
    GridBounds::containing(self.bounds(time))
  }
}

#[derive(Debug)]
pub struct TypedMoverView<'a, T> {
  mover: &'a mut Mover,
  _marker: PhantomData<T>,
}

impl<'a, T> TypedMoverView<'a, T> {
  fn behavior<'b>(&'b self) -> &'b T
  where
    &'b T: TryFrom<&'b MoverBehavior>,
  {
    match (&self.mover.behavior).try_into() {
      Ok(b) => b,
      Err(_) => panic!("A TypedMoverView existed for the wrong type"),
    }
  }
  fn behavior_mut<'b>(&'b mut self) -> &'b mut T
  where
    &'b mut T: TryFrom<&'b mut MoverBehavior>,
  {
    match (&mut self.mover.behavior).try_into() {
      Ok(b) => b,
      Err(_) => panic!("A TypedMoverView existed for the wrong type"),
    }
  }
}

impl<'a, T> Deref for TypedMoverView<'a, T> {
  type Target = Mover;

  fn deref(&self) -> &Mover {
    self.mover
  }
}

impl<'a, T> DerefMut for TypedMoverView<'a, T> {
  fn deref_mut(&mut self) -> &mut Mover {
    &mut self.mover
  }
}

#[derive(Debug)]
pub struct MoverUpdateContext<'a> {
  pub id: MoverId,
  pub game: &'a mut Game,
}

#[derive(Debug)]
pub struct MoverImmutableContext<'a> {
  pub id: MoverId,
  pub game: &'a Game,
}

impl<'a> MoverUpdateContext<'a> {
  pub fn this(&self) -> &Mover {
    self.game.mover(self.id).unwrap()
  }
  pub fn mutate_this<T, R, F: FnOnce(TypedMoverView<T>) -> R>(&mut self, f: F) -> R {
    self
      .game
      .mutate_mover(self.id, |mover| {
        f(TypedMoverView {
          mover,
          _marker: PhantomData,
        })
      })
      .unwrap()
  }
  // take self by value unnecessarily, to protect from accidentally doing stuff after destroyed
  pub fn destroy_this(self) {
    self.game.remove_mover(self.id);
  }
}

impl<'a> MoverImmutableContext<'a> {
  pub fn this(&self) -> &Mover {
    self.game.mover(self.id).unwrap()
  }
}

#[allow(unused)]
pub trait MoverBehaviorTrait {
  /** Perform a single scheduled update on this mover, possibly modifying the game state.

  Note that when called, `self` is a *copy* of the actual MoverBehavior implementor;
  use `context.mutate_this()` to change this Mover.
  MoverBehavior implementors are currently expected to have no data; they use the shared data that is in Mover.
  */
  fn wake(&self, context: MoverUpdateContext) {}
  fn next_wake(&self, this: &Mover) -> Option<Time> {
    None
  }

  fn collide(&self, context: MoverCollideContext) {}

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
    MoverBehavior::Material(Material)
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Monster {
  pub home: FloatingVector,
  pub active_time: Range<f64>,
  pub next_wake: Time,
}

const MONSTER_WAKE_DELAY: Time = 0.1;

impl MoverBehaviorTrait for Monster {
  fn wake(&self, mut context: MoverUpdateContext) {
    let active = self.active_time.contains(&context.game.day_progress);
    let target;
    if active {
      target = FloatingVector::zeros();
    } else {
      target = self.home;
    }

    let relative_target = target - context.this().position(context.game.physics_time);

    let acceleration = auto_constant("monster_acceleration", 4.0) * TILE_WIDTH as f64;
    let max_speed = auto_constant("monster_max_speed", 1.6) * TILE_WIDTH as f64;
    // Account for stopping distance:
    let target_speed = max_speed.min((2.0 * relative_target.magnitude() * acceleration).sqrt());
    let target_velocity = relative_target.normalize() * target_speed;
    context.mutate_this(|this: TypedMoverView<Self>| {
      this
        .velocity
        .move_towards(target_velocity, acceleration * MONSTER_WAKE_DELAY);

      this.behavior().next_wake = context.game.physics_time + MONSTER_WAKE_DELAY;
    });
  }

  fn next_wake(&self, _this: &Mover) -> Option<Time> {
    Some(self.next_wake)
  }

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.this().position(context.game.physics_time),
      TILE_SIZE.to_floating() * 0.8,
      "#000",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Projectile {
  pub disappear_time: Time,
}

impl MoverBehaviorTrait for Projectile {
  fn wake(&self, mut context: MoverUpdateContext) {
    context.destroy_this();
  }

  fn next_wake(&self, _this: &Mover) -> Option<Time> {
    Some(self.disappear_time)
  }

  fn collide(&self, context: MoverCollideContext) {
    if context.other().mover_type == MoverType::Monster {
      let impact = auto_constant("projectile_impact", 2.0) * TILE_WIDTH as f64;

      // Push the monster directly away from your deck. We COULD have the monster be propelled in the direction of the projectile (context.this().velocity) instead, but that causes the monster to be knocked around in a way that feels slippery, rather than feeling like a struggle of determination against determination. Having projectile directions matter would make this a physics game, and I actually DON'T want this to be a physics game.
      context
        .mutate_other(|target| {
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
      context.this().position(context.game.physics_time),
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
      context.this().position(context.game.physics_time),
      TILE_SIZE.to_floating() * 0.25,
      "#fff",
    );
  }
}
