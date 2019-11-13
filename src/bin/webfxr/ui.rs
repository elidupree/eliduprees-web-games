use std::cell::RefCell;
use std::rc::Rc;
use stdweb::Value;

use super::*;


pub fn undo() {
  let sound_changed = with_state_mut(|state| {
    if state.undo_position > 0 {
      state.undo_position -= 1;
      state.sound = state.undo_history[state.undo_position].clone();
      true
    }
    else {
      false
    }
  });
  if sound_changed {
    update_for_changed_sound();
  }
}
pub fn redo() {
  let sound_changed = with_state_mut(|state| {
    if state.undo_position + 1 < state.undo_history.len() {
      state.undo_position += 1;
      state.sound = state.undo_history[state.undo_position].clone();
      true
    }
    else {
      false
    }
  });
  if sound_changed {
    update_for_changed_sound();
  }
}




/*pub enum SamplesCanvasKind {
  Signal,
  Rendered,
}*/

pub struct IllustrationCanvas {
  pub canvas_id: String,
  //pub kind: SamplesCanvasKind,
  pub lines_drawn: usize,
  pub getter: DynamicGetter<RenderingState, Illustration>,
}


impl IllustrationCanvas {
  pub fn new(
    id: String,
    getter: DynamicGetter<RenderingState, Illustration>,
  ) -> IllustrationCanvas {
    IllustrationCanvas {
      canvas_id: id,
      lines_drawn: 0,
      getter: getter,
    }
  }
  pub fn draw_line(&self, illustration: &Illustration, index: usize) {
    let line = &illustration.lines[index];

    js! {
      var canvas = document.getElementById (@{& self.canvas_id});
      var context = canvas.getContext ("2d");

      context.fillStyle = @{line.clipping} ? "rgb(255,0,0)" : "rgb(0,0,0)";

      context.fillRect (@{index as f64}, canvas.height*(1 -@{line.range [1]})-0.5, 1, canvas.height*@{line.range [1] - line.range [0]}+1.0);
    }
  }

  pub fn draw_next_line(&mut self, illustration: &Illustration) {
    self.draw_line(illustration, self.lines_drawn);
    self.lines_drawn += 1;
  }

  /*pub fn reset(&self) {
    js! {
      var canvas = document.getElementById (@{& self.canvas_id});
      var context = canvas.getContext ("2d");

      context.clearRect (0, 0, canvas.width, canvas.height);
    }
  }*/

  pub fn update(&mut self) {
    with_state(|state| {
      let illustration = self.getter.get(&state.rendering_state);
      //println!("{:?}", (self.lines_drawn, illustration.lines.len()));
      while self.lines_drawn < illustration.lines.len() {
        self.draw_next_line(illustration);
      }
    });
  }

  pub fn redraw(
    &mut self,
    playback_position: Option<f64>,
    constants: &RenderingStateConstants,
  ) {
    //self.reset();
    self.update();

    if let Some(playback_position) = playback_position {
      let index = (playback_position * constants.sample_rate as f64
        / constants.samples_per_illustrated as f64)
        .floor();
      js! {
        var canvas = document.getElementById (@{& self.canvas_id});
        var context = canvas.getContext ("2d");

        context.fillStyle = "rgb(255,255,0)";
        context.fillRect (@{index as f64}, 0, 1, canvas.height);
      }
    }
  }
}



pub fn make_rendered_canvas<
  Builder: UIBuilder,
  G: Clone + 'static + GetterBase<From = RenderingState, To = RenderedSamples>,
>(
  builder: &mut Builder,
  id: &str,
  rendered_getter: Getter<G>,
  height: i32,
) -> Element {
  
  let canvas = Rc::new (RefCell::new (IllustrationCanvas::new(
    id.to_string(),
    (rendered_getter.clone()
      + getter! (samples: RenderedSamples => Illustration {samples.illustration}))
    .dynamic(),
  )));

  {
   let getter = rendered_getter.clone();
  builder.add_event_listener(id.to_string(), move | _:ClickEvent | {
    play (getter.clone());
  });
  
  }
    {canvas = canvas.clone();
  builder.after_morphdom (move | | {
    //rendered.redraw (None, & guard.rendering_state.constants);
    //canvas.reset();
    canvas.borrow_mut().update();
  });}
  
  builder.on_render_progress (move | | {
    canvas.borrow_mut().update();
  });
  
  html!{
    <canvas id=id width={(MAX_RENDER_LENGTH*DISPLAY_SAMPLE_RATE) as usize} height={height as usize} />
  }
}
