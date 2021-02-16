use super::*;

use boolinator::Boolinator;
use rand::{Rng, SeedableRng};

use std::cell::Cell;
use std::rc::Rc;
use std::str::FromStr;

pub const VISIBLE_LENGTH: f64 = 2.4;
pub const WIDTH_AT_CLOSEST: f64 = 0.5;
#[derive(Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default)]
pub struct Constants {
  #[derivative(Default(value = "1200"))]
  pub visible_components: i32,
  #[derivative(Default(value = "VISIBLE_LENGTH"))]
  pub visible_length: f64,
  #[derivative(Default(value = "CylindricalPerspective {
        width_at_closest: WIDTH_AT_CLOSEST,
        camera_distance_along_tangent: 0.11,
        radians_visible: 0.1,
        horizon_drop: 0.36,
      }"))]
  pub perspective: CylindricalPerspective,

  #[derivative(Default(value = "0.16"))]
  pub player_position: f64,
  #[derivative(Default(value = "0.1"))]
  pub player_max_speed: f64,

  #[derivative(Default(value = "WIDTH_AT_CLOSEST*0.5 + VISIBLE_LENGTH*2.0"))]
  pub spawn_radius: f64,
  #[derivative(Default(value = "VISIBLE_LENGTH*0.8"))]
  pub spawn_distance: f64,

  #[derivative(Default(value = "35.0"))]
  pub mountain_spawn_radius: f64,
  #[derivative(Default(value = "35.0"))]
  pub mountain_spawn_distance: f64,
  #[derivative(Default(value = "5.0"))]
  pub mountain_viewable_distances_radius: f64,
  #[derivative(Default(value = "0.10"))]
  pub mountain_density: f64,

  #[derivative(Default(value = "0.47"))]
  pub monster_density: f64,
  #[derivative(Default(value = "9.0"))]
  pub tree_density: f64,
  #[derivative(Default(value = "1.5"))]
  pub chest_density: f64,
  #[derivative(Default(value = "1.5"))]
  pub reward_density: f64,

  #[derivative(Default(value = "VISIBLE_LENGTH*0.2"))]
  pub fadein_distance: f64,

  #[derivative(Default(value = "0.25"))]
  pub speech_fade_duration: f64,
  #[derivative(Default(value = "3.5"))]
  pub speech_duration: f64,

  #[derivative(Default(value = "3.2"))]
  pub fall_duration: f64,

  #[derivative(Default(value = "1.2"))]
  pub chest_open_duration: f64,
}
js_serializable!(Constants);
js_deserializable!(Constants);

#[derive(Debug)]
pub struct Mountain {
  pub fake_peak_location: Vector3,
  pub base_screen_radius: f64,
}
#[derive(Debug)]
pub struct Sky {
  pub screen_position: Vector2,
  pub steepness: f64,
}
#[derive(Debug, Derivative)]
#[derivative(Default)]
pub struct Object {
  #[derivative(Default(value = "Vector2::new(0.0,0.0)"))]
  pub center: Vector2,
  #[derivative(Default(value = "Vector2::new(0.0,0.0)"))]
  pub velocity: Vector2,
  pub radius: f64,
  pub statements: Vec<Statement>,
  pub last_statement_start_time: Option<f64>,
  pub falling: Option<Fall>,
  pub collect_progress: f64,
  pub creation_progress: f64,

  pub automatic_statements: Vec<AutomaticStatement>,
  #[derivative(Default(value = "Kind::Tree"))]
  pub kind: Kind,
}
#[derive(Debug)]
pub struct Statement {
  pub text: String,
  pub start_time: f64,
  pub response: Option<String>,
  pub direction: Cell<f64>,
}
#[derive(Debug)]
pub struct AutomaticStatement {
  pub text: String,
  pub distances: [f64; 2],
  pub last_stated: Option<f64>,
}
#[derive(Debug)]
pub enum Kind {
  Person(Person),
  Chest,
  Reward,
  Monster(Monster),
  Tree,
}
#[derive(Debug)]
pub struct Monster {
  pub attack_progress: f64,
  pub attack_direction: f64,
  pub eye_direction: Vector2,
}
#[derive(Debug)]
pub struct Person {
  pub planted_foot: usize,
  pub feet: [Vector2; 2],
}
#[derive(Clone, Debug)]
pub struct Fall {
  pub progress: f64,
  pub distance: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Path {
  pub components: Vec<Component>,
  pub max_speed: f64,
  pub radius: f64,
}
#[derive(Clone, Debug)]
pub struct Component {
  pub center: Vector2,

  // per unit distance forward
  pub velocity: f64,
  pub acceleration: f64,
}
#[derive(Debug)]
pub struct Click {
  pub location: Vector2,
  pub player_location: Vector2,
  pub distance_traveled: f64,
  pub time: f64,
}

pub type Generator = ::rand_xoshiro::Xoshiro256PlusPlus;

#[derive(Derivative)]
#[derivative(Default)]
pub struct State {
  pub mountains: Vec<Mountain>,
  pub skies: Vec<Sky>,
  pub objects: Vec<Object>,
  pub path: Path,
  pub player: Object,
  pub companion: Object,

  pub permanent_pain: f64,
  pub permanent_pain_smoothed: f64,
  pub temporary_pain: f64,
  pub temporary_pain_smoothed: f64,

  pub last_click: Option<Click>,
  pub distance_traveled: f64,

  pub stars_collected: i32,

  #[derivative(Default(value = "Generator::from_seed([1; 32])"))]
  pub generator: Generator,
  pub constants: Rc<Constants>,
  pub now: f64,
}

impl Fall {
  pub fn info(&self, constants: &Constants, velocity: Vector2) -> (Vector2, f64) {
    let hit_ground = auto_constant("hit_ground", 0.3);
    let start_getting_up = auto_constant("start_getting_up", 1.5);
    let start_moving_fraction = auto_constant("start_moving_fraction", 0.1);
    let finish = constants.fall_duration;
    let fallen_angle = self.distance.signum() * TURN / 4.0;
    if self.progress < hit_ground {
      let fraction = self.progress / hit_ground;
      (
        Vector2::new(self.distance / hit_ground, 0.0),
        fraction * fraction * fallen_angle,
      )
    } else if self.progress < start_getting_up {
      (Vector2::new(0.0, 0.0), fallen_angle)
    } else {
      let fraction = (self.progress - start_getting_up) / (finish - start_getting_up);
      let velocity_factor = if fraction < start_moving_fraction {
        0.0
      } else {
        (fraction - start_moving_fraction) / (1.0 - start_moving_fraction)
      };
      let steepness = auto_constant("rise_steepness", 4.0);
      let angle_frac = fraction * fraction * fraction * fraction * 0.0
        + 4.0 * fraction * fraction * fraction * (1.0 - fraction) * 0.0
        + 6.0
          * fraction
          * fraction
          * (1.0 - fraction)
          * (1.0 - fraction)
          * ((0.5 - fraction) * steepness + 0.5)
        + 4.0 * fraction * (1.0 - fraction) * (1.0 - fraction) * (1.0 - fraction) * 1.0
        + (1.0 - fraction) * (1.0 - fraction) * (1.0 - fraction) * (1.0 - fraction) * 1.0;
      (velocity * velocity_factor, angle_frac * fallen_angle)
    }
  }
}

impl Object {
  pub fn say(&mut self, statement: Statement) {
    self.last_statement_start_time = Some(statement.start_time);
    self.statements.push(statement);
  }
  pub fn move_object(&mut self, movement: Vector2) {
    self.center += movement;
    match self.kind {
      Kind::Person(ref mut person) => {
        // hack: subdivide movement to reduce error in foot switching
        let mut perpendicular = Vector2::new(movement[1], -movement[0]);
        let norm = perpendicular.norm();
        if norm != 0.0 {
          perpendicular /= norm;
          let limit = self.radius * auto_constant("feet_motion_limit", 0.8);
          let parts = (movement.norm() * 10.0 / limit).ceil();
          for _ in 0..parts as usize {
            let planted_foot = person.planted_foot;
            let moving_foot = 1 - planted_foot;
            person.feet[moving_foot] += movement / parts;
            person.feet[planted_foot] -= movement / parts;

            let mut perpendicular_component_size = person.feet[moving_foot].dot(&perpendicular);
            if perpendicular_component_size < 0.0 {
              perpendicular_component_size = -perpendicular_component_size;
              perpendicular = -perpendicular;
            }
            let mut adjustment_size = (movement.norm() / parts)
              * auto_constant("feet_perpendicular_adjustement_ratio", 0.4);
            if adjustment_size > perpendicular_component_size {
              adjustment_size = perpendicular_component_size;
            }
            person.feet[moving_foot] -= perpendicular * adjustment_size;
            if person.feet[moving_foot].norm() > limit
              && person.feet[moving_foot].dot(&movement) > 0.0
            {
              person.planted_foot = moving_foot;
            }
          }
        }
      }
      _ => (),
    }
  }
}

impl Path {
  pub fn extend(
    &mut self,
    until: f64,
    player_horizontal: f64,
    generator: &mut Generator,
    constants: &Constants,
  ) {
    while self.components.last().unwrap().center[1] < until {
      let previous = self.components.last().unwrap().clone();
      let distance = constants.visible_length / constants.visible_components as f64;
      let mut new = Component {
        center: previous.center + Vector2::new(distance * previous.velocity, distance),
        velocity: previous.velocity + distance * previous.acceleration,
        acceleration: previous.acceleration,
      };

      let default_acceleration_change_radius = self.max_speed * 216.0 * distance;
      let mut bias = -previous.velocity * 36.0 * distance;
      // The path secretly follows the player if the player moves too far away,
      // for both gameplay and symbolism reasons.
      let player_offset = player_horizontal - previous.center[0];
      if player_offset > 0.7 {
        bias += (player_offset - 0.7) * self.max_speed * 40.0 * distance;
      }
      if player_offset < -0.7 {
        bias += (player_offset + 0.7) * self.max_speed * 40.0 * distance;
      }

      // can move faster to catch up
      let hard_max = self.max_speed * max(1.0, min(2.5, player_offset.abs()));

      let limits_1 = [
        previous.acceleration - default_acceleration_change_radius + bias,
        previous.acceleration + default_acceleration_change_radius + bias,
      ];
      // It's forbidden to accelerate to higher than max speed.
      // To keep things smooth, we never accelerate more than a fraction of the way to max speed at a time.
      // TODO: make this formula less dependent on the component size
      let limits_2 = [
        (-hard_max - previous.velocity) * 200.0,
        (hard_max - previous.velocity) * 200.0,
      ];
      let acceleration_limits = [max(limits_1[0], limits_2[0]), min(limits_1[1], limits_2[1])];

      //println!("{:?}", (limits_1, limits_2, acceleration_limits));
      if acceleration_limits[0] < acceleration_limits[1] {
        new.acceleration = generator.gen_range(acceleration_limits[0], acceleration_limits[1]);
      } else {
        new.acceleration = (acceleration_limits[0] + acceleration_limits[1]) / 2.0;
      }

      self.components.push(new);
    }
  }
}

impl State {
  pub fn do_spawns_impl<G: FnMut(f64, &mut Generator) -> f64, F: FnMut() -> Object>(
    &mut self,
    advance_distance: f64,
    average_number: f64,
    mut horizontal_position_generator: G,
    mut object_generator: F,
  ) {
    let attempts = (average_number * 10.0).ceil() as usize;
    for _ in 0..attempts {
      if self.generator.gen::<f64>() < average_number / attempts as f64 {
        let mut object = (object_generator)();
        let vertical_position = self.player.center[1] + self.constants.spawn_distance
          - self.generator.gen_range(0.0, advance_distance);
        object.center = Vector2::new(
          (horizontal_position_generator)(vertical_position, &mut self.generator),
          vertical_position,
        );
        self.objects.push(object);
      }
    }
  }
  pub fn do_spawns<F: FnMut() -> Object>(
    &mut self,
    advance_distance: f64,
    density: f64,
    object_generator: F,
  ) {
    let spawn_area = advance_distance * self.constants.spawn_radius * 2.0;
    let average_number = spawn_area * density;
    let radius = self.constants.spawn_radius;
    let player_center = self.player.center[0];
    self.do_spawns_impl(
      advance_distance,
      average_number,
      |_, generator| player_center + generator.gen_range(-radius, radius),
      object_generator,
    );
  }
  pub fn mountain_screen_peak(&self, mountain: &Mountain) -> Vector2 {
    let distance = mountain.fake_peak_location[1] - self.player.center[1];
    let highest_distance =
      self.constants.mountain_spawn_distance - self.constants.mountain_viewable_distances_radius;
    let distance_from_highest =
      (distance - highest_distance).abs() / self.constants.mountain_viewable_distances_radius;
    Vector2::new(
      (mountain.fake_peak_location[0] - self.player.center[0]) / distance,
      self.constants.perspective.horizon_drop
        - (mountain.fake_peak_location[2]) * (distance_from_highest * (TURN / 4.0)).cos(),
    )
  }
  pub fn spawn_mountains(&mut self, advance_distance: f64) {
    let spawn_area = advance_distance * self.constants.mountain_spawn_radius * 2.0;
    let average_number = spawn_area * self.constants.mountain_density;
    let attempts = (average_number * 10.0).ceil() as usize;
    for _ in 0..attempts {
      if self.generator.gen::<f64>() < average_number / attempts as f64 {
        let location = Vector3::new(
          self.player.center[0]
            + self.generator.gen_range(
              -self.constants.mountain_spawn_radius,
              self.constants.mountain_spawn_radius,
            ),
          self.player.center[1] + self.constants.mountain_spawn_distance
            - self.generator.gen_range(0.0, advance_distance),
          self.generator.gen_range(0.1, 0.2),
        );
        let mountain = Mountain {
          fake_peak_location: location,
          base_screen_radius: self.generator.gen_range(0.3, 0.6),
        };
        let screen_peak = self.mountain_screen_peak(&mountain)[0];
        let partial_radius = mountain.base_screen_radius * 0.8;
        if screen_peak + partial_radius < 0.0 || 0.0 < screen_peak - partial_radius {
          self.mountains.push(mountain);
        }
      }
    }
  }

  pub fn simulate(&mut self, duration: f64) {
    let tick_start = self.now;
    self.now += duration;
    let now = self.now;
    let constants = self.constants.clone();
    for sky in self.skies.iter_mut() {
      sky.screen_position[0] += 0.005 * duration * self.generator.gen_range(-1.0, 1.0);
      sky.screen_position[1] += 0.005 * duration * self.generator.gen_range(-1.0, 1.0);
      sky.screen_position[0] -= (sky.screen_position[0]) * 0.00006 * duration;
      sky.screen_position[1] -= (sky.screen_position[1]
        - 0.7 * self.constants.perspective.horizon_drop)
        * 0.00003
        * duration;
    }

    // hack: initialization stuff
    if tick_start == 0.0 {
      // hack: make the player start on the path even though the path isn't generated yet
      let (_min_visible_position, max_visible_position) = self.visible_range();
      self.path.extend(
        max_visible_position,
        self.player.center[0],
        &mut self.generator,
        &constants,
      );
      self.player.center[0] = self.path.horizontal_center(self.player.center[1]);
      self.companion.center[0] = self.path.horizontal_center(self.companion.center[1]);

      // generate as many mountains as would be visible
      self.spawn_mountains(constants.mountain_viewable_distances_radius * 2.0);
    }

    let movement_direction = if let Some(click) = self.last_click.as_ref() {
      click.location - click.player_location
    } else {
      Vector2::new(0.0, 1.0)
    };
    self.player.velocity =
      movement_direction * constants.player_max_speed / movement_direction.norm();

    if let Some(ref fall) = self.player.falling {
      let (velocity, _) = fall.info(&constants, self.player.velocity);
      self.player.velocity = velocity;
    }

    self.player.falling = self.player.falling.take().and_then(|mut fall| {
      fall.progress += duration;
      (fall.progress < constants.fall_duration).as_some(fall)
    });

    let mut time_moved = duration;
    let mut collisions = Vec::new();
    for (index, object) in self.objects.iter().enumerate() {
      if match object.kind {
        Kind::Monster(Monster {
          attack_progress, ..
        }) if attack_progress > 0.0 => true,
        _ => false,
      } {
        continue;
      }
      if object.creation_progress > 0.0 {
        continue;
      }
      let relative_velocity = object.velocity - self.player.velocity;
      let relative_location = object.center - self.player.center;
      if relative_velocity[1] < 0.0 && relative_location[1] > 0.0 {
        let time_to_collide = -relative_location[1] / relative_velocity[1];
        let horizontal_difference_when_aligned =
          relative_location[0] + relative_velocity[0] * time_to_collide;
        if horizontal_difference_when_aligned.abs() < (object.radius + self.player.radius) * 0.9 {
          if time_to_collide < time_moved {
            time_moved = time_to_collide - 0.0001;
          }
          if time_to_collide < duration {
            collisions.push(index);
          }
        }
      }
    }

    let movement_vector = self.player.velocity * time_moved;
    let advance_distance = movement_vector[1];

    self.player.move_object(movement_vector);
    self.distance_traveled += movement_vector.norm();

    let player_center = self.player.center;
    let (min_visible_position, max_visible_position) = self.visible_range();

    self.spawn_mountains(advance_distance);
    self.mountains.retain(|mountain| {
      let distance = mountain.fake_peak_location[1] - player_center[1];
      distance
        > constants.mountain_spawn_distance - constants.mountain_viewable_distances_radius * 2.0
    });
    self.path.extend(
      max_visible_position,
      self.player.center[0],
      &mut self.generator,
      &constants,
    );
    self.path.components.retain(|component| {
      component.center[1]
        >= min_visible_position - constants.visible_length / constants.visible_components as f64
    });

    // hack: monsters pursuing the player means that it's disproportionately likely that the first thing you'll encounter is a monster. I want you to generally meet the other game mechanics before you meet monsters. So make monsters less frequent early on.
    let monster_density =
      constants.monster_density * min(1.0, player_center[1] / (constants.visible_length * 2.0));

    self.do_spawns(advance_distance, constants.tree_density, || Object {
      kind: Kind::Tree,
      radius: 0.05,
      ..Default::default()
    });
    self.do_spawns(advance_distance, monster_density, || Object {
      kind: Kind::Monster(Monster {
        attack_progress: 0.0,
        attack_direction: 0.0,
        eye_direction: Vector2::new(0.0, 0.0),
      }),
      radius: 0.05,
      ..Default::default()
    });
    self.do_spawns(advance_distance, constants.chest_density, || Object {
      kind: Kind::Chest,
      radius: 0.03,
      ..Default::default()
    });
    self.do_spawns(advance_distance, constants.reward_density, || Object {
      kind: Kind::Reward,
      radius: 0.03,
      ..Default::default()
    });

    {
      // hack-ish: chests and rewards appear more frequently on or near the path.
      // Symbolism-wise, it should be the path that slightly steers towards rewards,
      // not the rewards that appear in the path.
      // But this way is simpler, and they're not very distinguishable to the player in practice.
      let hack = self.path.clone();
      let factor = auto_constant("path_reward_frequency_factor", 0.1);
      self.do_spawns_impl(
        advance_distance,
        advance_distance * constants.reward_density * factor,
        |vertical, generator| {
          hack.horizontal_center(vertical)
            + generator.gen_range(-hack.radius, hack.radius)
            + generator.gen_range(-hack.radius, hack.radius)
        },
        || Object {
          kind: Kind::Reward,
          radius: 0.03,
          ..Default::default()
        },
      );
      self.do_spawns_impl(
        advance_distance,
        advance_distance * constants.chest_density * factor,
        |vertical, generator| {
          hack.horizontal_center(vertical)
            + generator.gen_range(-hack.radius, hack.radius)
            + generator.gen_range(-hack.radius, hack.radius)
        },
        || Object {
          kind: Kind::Reward,
          radius: 0.03,
          ..Default::default()
        },
      );
    }

    for object in self.objects.iter_mut() {
      object.center += object.velocity * duration;
      if object.creation_progress > 0.0 {
        object.creation_progress = max(
          0.0,
          object.creation_progress - duration / constants.chest_open_duration,
        );
        continue;
      }

      match object.kind {
        Kind::Monster(ref mut monster) => {
          if monster.attack_progress == 0.0 || monster.attack_progress >= 1.0 {
            let speed_limit = 0.1;
            let mut acceleration = random_vector_within_length(&mut self.generator, 0.1);
            monster.eye_direction += safe_normalize(object.velocity) * duration;
            monster.eye_direction = safe_normalize(monster.eye_direction);
            if object.center[1] > player_center[1] {
              let player_attack_location = Vector2::new(
                player_center[0],
                object.center[1] / 2.0 + player_center[1] / 2.0,
              );
              let player_attack_vector = player_attack_location - object.center;
              let angle = player_attack_vector[0].atan2(-player_attack_vector[1]);
              if angle.abs() < TURN / 5.0
                && player_attack_vector.norm() < auto_constant("monster_attack_range", 0.4)
              {
                acceleration = safe_normalize(player_attack_vector) * 0.8;

                let player_offset = player_center - object.center;
                monster.eye_direction += safe_normalize(player_offset)
                  * duration
                  * auto_constant("monster_player_eye_focus_factor", 15.0);
                monster.eye_direction = safe_normalize(monster.eye_direction);
              }
            }

            object.velocity += acceleration * duration;
            if object.velocity.norm() > speed_limit {
              object.velocity *= 0.5f64.powf(duration / 0.1);
            }

            // hack: monsters going backwards near the player kind of breaks the game mechanics.
            // So make sure monsters are always going forwards once they get to the player,
            // but try to smooth it out at least a bit.
            let max_backwards = max(-0.01, object.center[1] - (player_center[1] + 0.01));
            object.velocity[1] = min(object.velocity[1], max_backwards);
          } else {
            monster.attack_progress += duration / auto_constant("monster_attack_duration", 0.12);
          }
        }
        Kind::Reward => {
          if object.collect_progress > 0.0 {
            object.collect_progress += duration * 0.7;
          }
        }
        Kind::Chest => {
          if object.collect_progress > 0.0 {
            object.collect_progress += duration / constants.chest_open_duration;
          }
        }
        _ => (),
      };
    }
    let mut companion_say = None;
    let player_distance_from_path =
      (self.path.horizontal_center(self.player.center[1]) - self.player.center[0]).abs()
        / self.path.radius;
    for object in self
      .objects
      .iter_mut()
      .chain(::std::iter::once(&mut self.player))
      .chain(::std::iter::once(&mut self.companion))
    {
      for statement in object.statements.iter_mut() {
        if now - statement.start_time > auto_constant("response_time", 1.0)
          && statement.response.is_some()
        {
          companion_say = statement.response.take();
        }
      }
      object
        .statements
        .retain(|statement| statement.start_time + constants.speech_duration > now);
    }

    let companion_movement_vector = Vector2::new(
      self
        .path
        .horizontal_center(self.companion.center[1] + advance_distance)
        - self.companion.center[0],
      advance_distance,
    );
    self.companion.move_object(companion_movement_vector);
    let companion_center = self.companion.center;

    let mut created_objects = Vec::new();
    for index in collisions {
      let object = &mut self.objects[index];
      let center_distance = self.player.center[0] - object.center[0];
      let direction = if center_distance < 0.0 { -1.0 } else { 1.0 };
      let minimal_escape_distance =
        (self.player.radius + object.radius) * direction - center_distance;
      match object.kind {
        Kind::Tree => {
          self.player.falling = Some(Fall {
            distance: minimal_escape_distance + self.player.radius * 0.25 * direction,
            progress: 0.0,
          });
          self.player.statements.push(Statement {
            text: String::from_str("Ow, it hurts").unwrap(),
            start_time: now,
            response: Some(
              String::from_str(if player_distance_from_path < 1.2 {
                "That's just part of life"
              } else {
                "It's your fault for straying"
              })
              .unwrap(),
            ),
            direction: Cell::new(direction),
          });
          self.temporary_pain += 1.0;
        }
        Kind::Reward => {
          if object.collect_progress == 0.0 {
            object.collect_progress = 0.000001;
            self.permanent_pain -= 0.10;
            self.player.statements.push(Statement {
              text: String::from_str("Yay!").unwrap(),
              start_time: now,
              response: Some(
                String::from_str(if player_distance_from_path < 1.2 {
                  "I'm proud of you"
                } else {
                  "That's not good for you"
                })
                .unwrap(),
              ),
              direction: Cell::new(1.0),
            });
          }
        }
        Kind::Chest => {
          if object.collect_progress == 0.0 {
            object.collect_progress = 0.000001;
            self.player.statements.push(Statement {
              text: String::from_str("What's inside?").unwrap(),
              start_time: now,
              response: None,
              direction: Cell::new(-1.0),
            });
            let mut new_object =
              if self.generator.gen::<f64>() < auto_constant("chest_monster_frequency", 0.25) {
                Object {
                  kind: Kind::Monster(Monster {
                    attack_progress: 0.0,
                    attack_direction: 0.0,
                    eye_direction: safe_normalize(Vector2::new(
                      center_distance / (self.player.radius + object.radius) * 2.0,
                      -1.0,
                    )),
                  }),
                  radius: 0.05,
                  ..Default::default()
                }
              } else {
                Object {
                  kind: Kind::Reward,
                  radius: 0.03,
                  ..Default::default()
                }
              };
            // note: make the creation progress be further along than the collection progress, because when this was just 1.0, there was a tiny leeway where the chest could disappear before the contents finished being created. And 50-milliseconds frames just HAPPENED to exactly hit that leeway
            new_object.creation_progress = 1.0 - 0.00001;
            new_object.center = object.center;
            new_object.center[1] += 0.0001;
            created_objects.push(new_object);
          }
        }
        Kind::Monster(ref mut monster) => {
          if monster.attack_progress == 0.0 {
            monster.attack_progress = 0.000001;
            self.permanent_pain += 0.22;
            self.temporary_pain += 1.4;
            monster.attack_direction = direction;
            object.velocity = Vector2::new(0.0, 0.0);
            self.player.falling = Some(Fall {
              distance: self.player.radius * 0.25 * direction, // minimal_escape_distance + self.player.radius*2.25 *direction,
              progress: 0.0,
            });
            self.player.statements.push(Statement {
              text: String::from_str("Ow, it hurts!").unwrap(),
              start_time: now,
              response: Some(
                String::from_str(if player_distance_from_path < 1.2 {
                  "Liar, that would never happen on the path"
                } else {
                  "It's your fault for straying"
                })
                .unwrap(),
              ),
              direction: Cell::new(direction),
            });
          }
        }
        Kind::Person(_) => unreachable!(),
      }
    }
    self
      .objects
      .retain(|object| object.center[1] > player_center[1] - 0.5 && object.collect_progress < 1.0);
    self.objects.extend(created_objects);

    self.permanent_pain_smoothed = self.permanent_pain
      + (self.permanent_pain_smoothed - self.permanent_pain)
        * 0.5f64.powf(duration / auto_constant("permanent_pain_smoothed_halflife", 0.7));
    self.temporary_pain = self.permanent_pain_smoothed
      + (self.temporary_pain - self.permanent_pain_smoothed)
        * 0.5f64.powf(duration / auto_constant("temporary_pain_halflife", 1.4));
    self.temporary_pain_smoothed = self.temporary_pain
      + (self.temporary_pain_smoothed - self.temporary_pain)
        * 0.5f64.powf(duration / auto_constant("temporary_pain_smoothed_halflife", 0.1));

    if companion_say.is_none() {
      for statement in self.companion.automatic_statements.iter_mut() {
        if self
          .companion
          .last_statement_start_time
          .map_or(true, |when| now > when + 5.0)
          && statement
            .last_stated
            .map_or(true, |when| now > when + 100.0)
        {
          let distance = player_distance_from_path;
          if distance >= statement.distances[0] && distance <= statement.distances[1] {
            statement.last_stated = Some(now);
            companion_say = Some(statement.text.clone());
            break;
          }
        }
      }
    }
    if let Some(companion_say) = companion_say {
      let virtual_location = player_center[0]
        + self
          .player
          .statements
          .iter()
          .map(|statement| statement.direction.get())
          .sum::<f64>()
          / (self.player.statements.len() as f64 + 0.01)
          * 0.15;
      let distance = companion_center[0] - virtual_location;
      let direction =
        if distance < 0.0 { 1.0 } else { -1.0 } * if distance.abs() < 0.25 { -1.0 } else { 1.0 };
      self.companion.say(Statement {
        text: companion_say,
        start_time: now,
        response: None,
        direction: Cell::new(direction),
      });
    }
  }
}
