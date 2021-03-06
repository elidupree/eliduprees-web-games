use super::*;

use ordered_float::OrderedFloat;
use stdweb::unstable::TryInto;

impl State {
  pub fn draw_object(&self, object: &Object, visible_radius: f64, speech_layer: bool) {
    let raw_position = as_ground(object.center);
    let position = self.draw_position(raw_position);
    let scale = self.draw_scale(raw_position);
    let scaled_radius = scale * object.radius;

    if !speech_layer && position[0].abs() > visible_radius + scaled_radius * 1.2 {
      return;
    }

    let alpha = max(
      0.0,
      min(
        1.0,
        (self.player.center[1] + self.constants.spawn_distance - object.center[1])
          / self.constants.fadein_distance,
      ),
    ) * (1.0 - object.creation_progress);
    js! {
      context.save();
      context.globalAlpha = @{alpha*(1.0 - object.collect_progress)};
    }

    match object.kind {
      Kind::Tree => {
        js! {
          var tree = tree_shape.clone({insert: false});
          tree.scale (@{scaled_radius}, [0,0]);
          tree.translate (@{position [0]}, @{position [1]});
          context.fillStyle = "rgb(70, 70, 70)";
          context.fill(new Path2D(tree.pathData));
        }
      }
      Kind::Reward => {
        js! {
          var reward = reward_shape.clone({insert: false});
          reward.rotate(@{360.0*object.collect_progress}, [0,0]);
          reward.translate (0,-reward_shape.bounds.bottom);
          reward.scale (@{scaled_radius}, [0,0]);
          reward.translate (@{position [0]}, @{position [1] - object.radius*2.0*object.collect_progress});
          var path = new Path2D(reward.pathData);
          context.fillStyle = "rgb(255, 255, 255)";
          context.strokeStyle = "rgb(0, 0, 0)";
          context.lineWidth = @{scaled_radius}*0.1;
          context.fill(path);
          context.stroke(path);
        }
      }
      Kind::Chest => {
        js! {
          var chest = chest_shape.clone({insert: false});
          chest.scale (@{scaled_radius}, [0,0]);
          chest.translate (@{position [0]}, @{position [1]});
          var path = new Path2D(chest.pathData);
          context.fillStyle = "rgb(255, 255, 255)";
          context.strokeStyle = "rgb(0, 0, 0)";
          context.lineWidth = @{scaled_radius}*0.1;
          context.fill(path);
          context.stroke(path);
        }
      }
      Kind::Monster(ref monster) => {
        let eyeball_height = object.radius * auto_constant("monster_eyeball_height", 1.3);
        let eyeball_radius = object.radius * auto_constant("monster_eyeball_radius", 0.27);
        let eyeball_pinching = object.radius * auto_constant("monster_eyeball_pinching", 0.22);
        let vertical_scale = (self.draw_position(raw_position + Vector3::new(0.0, 0.0001, 0.0))[1]
          - position[1])
          / 0.0001;
        let both_scale = Vector2::new(scale, vertical_scale).norm();

        let directions = [-1.0, 1.0];
        for &direction in directions.iter() {
          let raw_eyeball_center = Vector3::new(
            object.center[0] + (object.radius - eyeball_radius - eyeball_pinching) * direction,
            object.center[1],
            eyeball_height,
          );
          let eyeball_center = self.draw_position(raw_eyeball_center);
          js! {
            context.fillStyle = "rgb(255, 255, 255)";
            context.strokeStyle = "rgb(0, 0, 0)";
            context.lineWidth = @{scale*object.radius}*0.04;
            context.beginPath();
            context.arc (@{eyeball_center[0]}, @{eyeball_center[1]}, @{eyeball_radius*scale}, 0, turn, true);
            context.fill(); context.stroke();
          }
          let pupil_offset = as_ground(monster.eye_direction * eyeball_radius) * scale / both_scale;
          let pupil_center = self.draw_position(raw_eyeball_center + pupil_offset);
          let pupil_visual_offset = pupil_center - eyeball_center;
          let pupil_radius = eyeball_radius * auto_constant("monster_pupil_radius", 1.0 / 3.0);
          js! {
            context.fillStyle = "rgb(0, 0, 0)";
            context.strokeStyle = "rgb(0, 0, 0)";
            context.lineWidth = @{pupil_radius*scale}*0.1;
            context.beginPath();
            var shape = new paper.Path.Ellipse ({center: [@{pupil_center [0]},@{pupil_center [1]}], radius: [@{pupil_radius*scale*(monster.eye_direction[1].abs()+0.00001)},@{pupil_radius*scale}], insert: false, });
            shape.rotate (@{pupil_visual_offset [1].atan2(pupil_visual_offset [0])*360.0/TURN });
            var path = new Path2D(shape.pathData);
            context.fill(path); context.stroke(path);
          }
        }

        js! {
          context.fillStyle = "rgba(100, 100, 100, 0.5)";
          var shape = new paper.Path.Ellipse ({center: [@{position [0]},@{position [1]}], radius: [@{scaled_radius},@{eyeball_radius*vertical_scale}], insert: false, });
          context.fill(new Path2D(shape.pathData));
        }

        if monster.attack_progress > 0.0 && monster.attack_progress < 1.0 {
          let claw_direction =
            Vector3::new(-1.0 * monster.attack_direction, 0.0, 0.3) * object.radius * 1.2;
          let claws_center = raw_position
            + Vector3::new(0.0, 0.0, eyeball_height / 2.0)
            + claw_direction * (1.0 - monster.attack_progress * 2.0);
          let claw_crosswise =
            Vector3::new(0.06 * monster.attack_direction, 0.0, 0.2) * object.radius;
          for index in -1..2 {
            let center = claws_center + claw_crosswise * index as f64 * 2.0;
            let tip_offset = claw_direction * 0.5;
            let tips = (center - tip_offset, center + tip_offset);
            let sides = (center - claw_crosswise, center + claw_crosswise);
            js! {
              context.fillStyle = "rgb(255, 255, 255)";
              context.strokeStyle = "rgb(0, 0, 0)";
              context.lineWidth = @{scaled_radius}*0.04;
              context.beginPath();
            }
            move_to(self.draw_position(tips.0));
            quadratic_curve(self.draw_position(sides.0), self.draw_position(tips.1));
            quadratic_curve(self.draw_position(sides.1), self.draw_position(tips.0));
            js! {
              context.fill(); context.stroke();
            }
          }
        }
      }
      Kind::Person(ref person) => {
        let mut rotation = 0.0;
        if let Some(ref fall) = object.falling {
          let (_, r) = fall.info(&self.constants, object.velocity);
          rotation = r;
        }
        let transformation1 = Rotation3::new(Vector3::new(0.0, rotation, 0.0));
        //let transformation2 = Rotation3::new (Vector3::new (0.0, 0.0, -rotation));
        let transformation = transformation1; //*transformation2;
        let transform = |vector: Vector3| transformation * vector;
        let body_base_vector = transform(Vector3::new(
          0.0,
          0.0,
          auto_constant("body_base_height", 1.0) * object.radius,
        ));
        let body_base = raw_position + body_base_vector;
        let body_peak = body_base
          + transform(Vector3::new(
            0.0,
            0.0,
            auto_constant("body_height", 2.0) * object.radius,
          ));
        let body_side_vector = transform(Vector3::new(object.radius, 0.0, 0.0));
        let leg_side_vector = transform(Vector3::new(
          auto_constant("leg_side", 11.0 / 24.0) * object.radius,
          0.0,
          0.0,
        ));
        let leg_inner_radius_vector = transform(Vector3::new(
          auto_constant("leg_inner_radius", 8.0 / 24.0) * object.radius,
          0.0,
          0.0,
        ));
        let leg_outer_radius_vector = transform(Vector3::new(
          auto_constant("leg_outer_radius", 7.0 / 24.0) * object.radius,
          0.0,
          0.0,
        ));
        let head_center = body_base
          + transform(Vector3::new(
            0.0,
            0.0,
            auto_constant("head_height", 1.7) * object.radius,
          ));
        let head_position = self.draw_position(head_center);
        let head_radius = auto_constant("head_radius", 0.7) * scaled_radius;

        if !speech_layer {
          js! {
            context.fillStyle = "rgb(255, 255, 255)";
            context.strokeStyle = "rgb(0, 0, 0)";
            context.lineWidth = @{scaled_radius}*0.1;
          }

          let mut feet = [(-1.0, &person.feet[0]), (1.0, &person.feet[1])];
          feet.sort_by_key(|foot| OrderedFloat(-foot.1[1]));
          for &(direction, foot) in feet.iter() {
            let foot = transform(Vector3::new(foot[0], foot[1], 0.0));
            js! { context.beginPath(); }
            move_to(
              self
                .draw_position(body_base + (leg_side_vector + leg_outer_radius_vector) * direction),
            );
            line_to(
              self
                .draw_position(body_base + (leg_side_vector - leg_inner_radius_vector) * direction),
            );
            line_to(self.draw_position(raw_position + leg_side_vector * direction + foot));
            js! { context.closePath(); context.fill(); context.stroke(); }
          }

          js! { context.beginPath(); }
          move_to(self.draw_position(body_peak));
          line_to(self.draw_position(body_base + body_side_vector));
          line_to(self.draw_position(body_base - body_side_vector));
          js! { context.closePath(); context.fill(); context.stroke(); }

          js! {
            context.beginPath();
            context.arc (@{head_position[0]}, @{head_position[1]}, @{head_radius}, 0, turn, true);
            context.fill(); context.stroke();
          }
        } else {
          // speech layer

          for statement in object.statements.iter() {
            let mut distortion = 0.0;
            let age = self.now - statement.start_time;
            let countdown = self.constants.speech_duration - age;
            let fade = self.constants.speech_fade_duration;
            if age < fade {
              distortion = (fade - age) / fade;
            }
            if countdown < fade {
              distortion = (countdown - fade) / fade;
            }

            let big_factor = 10000.0;

            js! {
              context.save();
              context.textBaseline = "middle";
              context.scale(0.0001,0.0001);
            }
            // try drawing, but sometimes we need to switch direction
            loop {
              let direction = statement.direction.get();
              let mut tail_tip_position = head_position
                + Vector2::new(
                  head_radius * auto_constant("speech_distance_from_head", 1.4) * direction,
                  0.0,
                );
              let limit = auto_constant("speech_position_limit", 0.005);
              let distance_below_limit = -visible_radius + limit - tail_tip_position[0] * direction;
              if distance_below_limit > 0.0 {
                tail_tip_position[0] += distance_below_limit * direction;
              }

              let text_height = auto_constant("text_height", 0.03) * big_factor;
              js! {
                context.font = @{text_height}+"px Arial, Helvetica, sans-serif";
              }
              let text_width: f64 = js! {
                return context.measureText (@{&statement.text}).width;
              }
              .try_into()
              .unwrap();

              let padding = text_height / 2.3 + text_width / 30.0;
              let bubble_left = -padding;
              let bubble_right = text_width + padding;
              let bubble_bottom = auto_constant("bubble_bottom", -0.016) * big_factor;
              let text_middle = bubble_bottom - padding - text_height / 2.0;
              let bubble_top = text_middle - padding - text_height / 2.0;

              let tail_left_join_x = auto_constant("tail_left_join_x", 0.017) * big_factor;
              let tail_right_join_x = auto_constant("tail_right_join_x", 0.03) * big_factor;

              if head_position[0] * direction > 0.0
                && tail_tip_position[0] * direction + bubble_right / big_factor > visible_radius
              {
                statement.direction.set(direction * -1.0);
                continue;
              }

              translate(tail_tip_position * big_factor);
              js! {
                context.rotate(@{distortion*TURN/17.0});
                context.scale(@{direction}, 1);
                context.globalAlpha = @{1.0 - distortion.abs()};

                context.beginPath();

              }

              move_to(Vector2::new(0.0, 0.0));
              quadratic_curve(
                Vector2::new(
                  tail_left_join_x,
                  auto_constant("tail_left_control_y", -0.005),
                ),
                Vector2::new(tail_left_join_x, bubble_bottom),
              );
              quadratic_curve(
                Vector2::new(bubble_left, bubble_bottom),
                Vector2::new(bubble_left, text_middle),
              );
              quadratic_curve(
                Vector2::new(bubble_left, bubble_top),
                Vector2::new(text_width * 0.5, bubble_top),
              );
              quadratic_curve(
                Vector2::new(bubble_right, bubble_top),
                Vector2::new(bubble_right, text_middle),
              );
              quadratic_curve(
                Vector2::new(bubble_right, bubble_bottom),
                Vector2::new(tail_right_join_x, bubble_bottom),
              );
              quadratic_curve(
                Vector2::new(
                  tail_right_join_x,
                  auto_constant("tail_right_control_y", -0.005),
                ),
                Vector2::new(0.0, 0.0),
              );
              js! {
                context.closePath();
                context.fillStyle = "rgb(255, 255, 255)";
                context.strokeStyle = "rgb(0, 0, 0)";
                context.lineWidth = @{auto_constant ("speech_stroke_width", 0.002)*big_factor};
                context.fill(); context.stroke();
                context.fillStyle = "rgb(0, 0, 0)";
              }
              if direction < 0.0 {
                js! {
                  context.scale(@{direction}, 1);
                  context.translate (@{- text_width}, 0);
                }
              }
              js! {
                context.fillText (@{&statement.text}, 0, @{text_middle});
              }
              break;
            }
            js! {context.restore();}
          }
        }
      }
      _ => {
        let first_corner = self.draw_position(Vector3::new(
          object.center[0] - object.radius,
          object.center[1],
          object.radius,
        ));
        let second_corner = self.draw_position(Vector3::new(
          object.center[0] + object.radius,
          object.center[1],
          0.0,
        ));
        let size = second_corner - first_corner;
        //println!("{:?}", (object, first_corner, second_corner, size));
        js! {
          context.fillStyle = "rgb(255,255,255)";
          context.fillRect (@{first_corner[0]}, @{first_corner[1]}, @{size[0]}, @{size[1]});
        }
      }
    };
    js! {
      context.restore();
    }
  }

  pub fn pain_radius(&self, pain: f64) -> f64 {
    let fraction = 0.5 - pain.atan() / (TURN / 2.0);
    // allow it to go a bit outside the boundaries of the screen,
    // don't allow it to reduce to a 0 size
    0.1 + fraction * 0.5
  }

  pub fn draw(&self, visible_radius: f64) {
    //let (min_visible_position, max_visible_position) = self.visible_range();

    let temporary_pain_radius = self.pain_radius(self.temporary_pain_smoothed);
    let actually_visible_radius = temporary_pain_radius * visible_radius * 2.0;

    let permanent_pain_speed = (self.permanent_pain_smoothed - self.permanent_pain).abs();
    if permanent_pain_speed > auto_constant("permanent_pain_threshold", 0.0001) {
      let permanent_pain_radius = self.pain_radius(self.permanent_pain_smoothed);
      js! {
        window.permanent_pain_ellipse = new paper.Path.Ellipse ({center: [0.0, 0.5], radius: [@{permanent_pain_radius*visible_radius*2.0},@{permanent_pain_radius}], insert: false, });
        context.lineWidth = @{permanent_pain_speed*auto_constant ("permanent_pain_factor", 0.025)};
        context.strokeStyle = "rgb(255,255,255)";
        context.stroke(new Path2D(permanent_pain_ellipse.pathData));
      }
    }

    let no_sky: bool = js! {return auto_constants.no_sky = auto_constants.no_sky || false}
      .try_into()
      .unwrap();
    if !no_sky {
      js! {
        window.temporary_pain_ellipse = new paper.Path.Ellipse ({center: [0.0, 0.5], radius: [@{actually_visible_radius},@{temporary_pain_radius}], insert: false, });

        context.save();
        context.clip(new Path2D(temporary_pain_ellipse.pathData));
      }
      let visible_sky = skyline(
        actually_visible_radius,
        &self
          .mountains
          .iter()
          .filter_map(|mountain| {
            let screen_peak = self.mountain_screen_peak(&mountain);
            let visible_base_radius = mountain.base_screen_radius
              * (self.constants.perspective.horizon_drop - screen_peak[1])
              / mountain.fake_peak_location[2];
            if screen_peak[0] + visible_base_radius < -actually_visible_radius
              || screen_peak[0] - visible_base_radius > actually_visible_radius
            {
              return None;
            }

            Some(ScreenMountain {
              peak: Vector2::new(
                screen_peak[0],
                self.constants.perspective.horizon_drop - screen_peak[1],
              ),
              radius: visible_base_radius,
            })
          })
          .collect::<Vec<_>>(),
      );
      js! {window.segments = [[@{- actually_visible_radius},0]];}
      for vector in visible_sky {
        js! {segments.push ([@{vector [0]},@{self.constants.perspective.horizon_drop-vector [1]}]);}
      }
      js! {
        segments.push ([@{actually_visible_radius},0]);
        var visible_sky = new paper.Path ({segments: segments, insert: false});
        visible_sky.closed = true;
        /*context.lineWidth = @{0.001};
        context.strokeStyle = "rgb(255,255,255)";
        context.stroke(new Path2D(visible_sky.pathData));*/
        visible_sky = visible_sky.intersect (temporary_pain_ellipse);

        context.save();
        context.clip(new Path2D(visible_sky.pathData));
      }
      for sky in self.skies.iter() {
        let pos = sky.screen_position;
        js! {
          var pos = [@{pos[0]}, @{pos[1]}];
          var visible_radius = @{visible_radius};
          var steepness = @{sky.steepness};
          var segments = [];
          segments.push([
              pos,
              [-0.4, 0],
              [0.4, 0]
            ]);
          segments.push([
              [Math.max (visible_radius, pos[0]+1.0), pos[1] + steepness],
              [-0.4, 0],
              [0, 0]
            ]);
          segments.push([Math.max (visible_radius, pos[0]+1.0), pos[1] + steepness + constants.perspective.horizon_drop]);
          segments.push([Math.min (-visible_radius, pos[0]-1.0), pos[1] + steepness + constants.perspective.horizon_drop]);
          segments.push([
              [Math.min (-visible_radius, pos[0]-1.0), pos[1] + steepness],
              [0, 0],
              [0.4, 0]
            ]);
          var sky = new paper.Path({ segments: segments, insert: false });
          sky.closed = true;
          context.fillStyle = "rgba(255,255,255, 0.05)";
          context.fill(new Path2D(sky.pathData));
        }
      }
      js! {context.restore();}
    }

    js! {
      context.beginPath();
    }
    let mut began = false;
    for component in self.path.components[0..self.path.components.len() - 1].iter() {
      let mut endpoint = self.draw_position(Vector3::new(
        component.center[0] - self.path.radius,
        component.center[1],
        0.0,
      ));

      // hack: work around a polygon display glitch that existed only in chromium, not Firefox
      if endpoint[0] < -visible_radius - 0.02 {
        endpoint[0] = -visible_radius - 0.02;
      }
      if endpoint[0] > visible_radius + 0.01 {
        endpoint[0] = visible_radius + 0.01;
      }

      if began {
        line_to(endpoint);
      } else {
        move_to(endpoint);
        began = true;
      }
    }
    /*{
      let last = &self.path.components[self.path.components.len()-2..self.path.components.len()];
      let distance = last [1].center - last [0].center;
      let horizon_distance = max_visible_position - last [0].center [1];
      let horizon_center = last [0].center + distance*horizon_distance/distance [1];
      let endpoint = self.draw_position (Vector3::new (horizon_center [0] - self.path.radius, max_visible_position, 0.0));
      line_to (endpoint);
      let endpoint = self.draw_position (Vector3::new (horizon_center [0] + self.path.radius, max_visible_position, 0.0));
      line_to (endpoint);
    }*/
    for component in self.path.components[0..self.path.components.len() - 1]
      .iter()
      .rev()
    {
      let mut endpoint = self.draw_position(Vector3::new(
        component.center[0] + self.path.radius,
        component.center[1],
        0.0,
      ));

      // hack: work around a polygon display glitch that existed only in chromium, not Firefox
      if endpoint[0] < -visible_radius - 0.01 {
        endpoint[0] = -visible_radius - 0.01;
      }
      if endpoint[0] > visible_radius + 0.02 {
        endpoint[0] = visible_radius + 0.02;
      }

      line_to(endpoint);
    }
    js! {
      context.fillStyle = "rgb(255,255,255)";
      context.fill();
    }

    if let Some(click) = self.last_click.as_ref() {
      let offset = click.location - click.player_location;
      let forward = offset / offset.norm();
      let perpendicular = Vector2::new(-forward[1], forward[0]);
      let segment_length = auto_constant("movement_segment_length", 0.025);
      let segment_period = 2.0 * segment_length;
      let segment_radius = auto_constant("movement_segment_radius ", 0.0025);

      let initial_offset = -self.player.radius - (self.distance_traveled % segment_period);

      let min_offset = -self.player.radius - segment_period;
      let max_offset = offset.norm() + segment_period - segment_length;
      let segments = 1.0 + ((max_offset - initial_offset) / segment_period).floor();

      let age = self.now - click.time;
      let fade_in = auto_constant("movement_fade_in", 0.2);
      let fade_out = auto_constant("movement_fade_out", 0.2);
      let alpha = if age < fade_in {
        age / fade_in
      } else {
        fade_out / (age - fade_in + fade_out)
      } * auto_constant("movement_variable_alpha", 0.1)
        + auto_constant("movement_fixed_alpha", 0.4);
      let brightness = (255.0 * auto_constant("movement_brightness", 0.7)).ceil();

      for index in 0..segments as usize {
        let offset = initial_offset + index as f64 * segment_period;
        let first = self.player.center + forward * offset;
        let second = first + forward * segment_length;
        let sin = ((offset - min_offset) / (max_offset - min_offset) * (TURN / 2.0)).sin();
        let alpha = alpha * min(sin * max(1.5, segments * 0.15), sin.sqrt());
        //if index == 0 {alpha *= 1.0 + initial_offset/segment_period;}
        //if index+1 == segments as usize {alpha *= - initial_offset/segment_period;}
        js! { context.beginPath(); }
        move_to(self.draw_position(as_ground(first - perpendicular * segment_radius)));
        line_to(self.draw_position(as_ground(first + perpendicular * segment_radius)));
        line_to(self.draw_position(as_ground(second + perpendicular * segment_radius)));
        line_to(self.draw_position(as_ground(second - perpendicular * segment_radius)));
        js! {
          context.fillStyle = "rgba("+@{brightness}+","+@{brightness}+","+@{brightness}+","+@{alpha}+")";
          context.fill();
        }
      }
    }

    let mut objects: Vec<_> = self.objects.iter().collect();
    objects.push(&self.player);
    objects.push(&self.companion);
    objects.sort_by_key(|object| OrderedFloat(-object.center[1]));
    for object in objects.iter() {
      self.draw_object(object, actually_visible_radius, false);
    }

    if !no_sky {
      js! { context.restore();}
    }

    self.draw_object(&self.player, visible_radius, true);
    self.draw_object(&self.companion, visible_radius, true);

    /*js! {
      context.save();
      context.textBaseline = "middle";
      context.scale(0.0001,0.0001);
      context.font = 500+"px Arial, Helvetica, sans-serif";
      context.fillStyle = "rgb(255, 255, 155)";
      context.fillText ("resizes: " + window.resizes + " canvas: "+ canvas.width + "x" + canvas.height + " window: "+ window.innerWidth + "x" + window.innerHeight, -@{visible_radius*9500.0}, 500);
      context.restore();
    }*/
  }
}
