use crate::UIBuilder;
use std::fs;
use std::path::Path;
use super::*;

struct StaticFilesUIBuilder {
  pub css: String,
  pub next_grid_row: i32,
}

impl UIBuilder for StaticFilesUIBuilder {
  fn css (&mut self, css: & str) {
    write!(&mut self.css, "{}\n", css);
  }
  fn next_grid_row_class (&mut self, classname: & str) {
    write!(&mut self.css, ".{classname} {{
  grid-row: {start} / {end}
}}
", classname=classname, start=self.next_grid_row, end=self.next_grid_row+1);
    self.next_grid_row += 1;
  }
  fn last_n_grid_rows_class (&mut self, classname: & str, n: i32) {
    write!(&mut self.css, ".{classname} {{
  grid-row: {start} / {end}
}}
", classname=classname, start=self.next_grid_row-n, end=self.next_grid_row);
  }
}

pub fn generate_static_files() {
  let mut builder = StaticFilesUIBuilder {
    css: "
    
".to_string(),
    next_grid_row: 0,
  };
  
  

  let name_column = 1;
  let dice_column = name_column + 1;
  let lock_column = dice_column + 1;
  let toggle_column = lock_column + 1;
  let input_column = toggle_column + 1;
  let add_effect_column = input_column + 1;
  let canvas_column = add_effect_column + 1;
  builder.css(&format!("
html,body {{background-color: white;}}

#app {{display: flex;}}
.left_column {{}}

.main_grid {{display: grid; grid-template-columns: 1fr 0fr 0fr 0fr 0fr 0fr 0fr 1fr;}}
.signal_toggle {{grid-column:{toggle_column}/ span 1; }}
.labeled_input.numeric, .effect_header, .odd_harmonics_toggle, .view_toggle {{grid-column: {input_column} / span 1; white-space: nowrap;}}
.sound_radio_input {{grid-column: {input_column} / span 2; }}
.add_effect_buttons {{grid-column: {add_effect_column} / span 1; width: 8em;}}
.signal_effect {{grid-column: {input_column} / span 2 !important; }}
.toplevel_input_label {{grid-column: {name_column} / span 1; text-align: right; align-self: center;}}
.input_region {{border: 0.125em solid #ccc; border-width: 0.0625em 0; grid-column: {name_column} / -1; }}
.input_region:nth-child(odd) {{background-color:#eee; }}
.main_grid .canvas_wrapper {{grid-column: {canvas_column} / span 1; align-self: center; }}
.panel {{margin:0.8em; padding:0.8em; background-color:#eee;}}
.panel .labeled_input {{margin:0.2em;}}
.panel label {{margin-left: 0.2em; margin-right: 0.6em;}}
input[type='checkbox'] {{width:2em;height:2em;}}
input[type='radio'] {{width:2em;height:2em;}}
input[type='button'] {{padding: 0 0.8em;}}
input[type='number'] {{width:5em;}}
input,select {{height:2em; vertical-align: middle; align-self: center;}}
label {{vertical-align: middle; align-self: center;}}
input[type='button'].down {{
  background-color: #bbb;
  background-image: linear-gradient(to top,#ddd,#999);
  border-color:#888 #777 #555 #888;
}}
.signal_not_applicable {{grid-column: {input_column} / span {not_applicable_span}; white-space: nowrap; font-style: italic; color: #555; padding:0.25em; }}

@media screen and (max-width: 30em) {{
}}
", name_column=name_column, 
//dice_column=dice_column, 
//lock_column=lock_column,
 toggle_column=toggle_column, input_column=input_column, add_effect_column=add_effect_column, canvas_column=canvas_column, not_applicable_span=canvas_column + 1 - input_column));
  
  let app_element = app(&mut builder);
  
  let args: Vec<_> = std::env::args().collect();
  let static_path = Path::new(&args[1]).join("static");
  
  
  
  fs::write(static_path.join("webfxr.html"), &format!(r#"<!DOCTYPE html>

<html>
<head>
    <meta charset="utf-8" />
    <link rel="stylesheet" type="text/css" href="/mimic-website.css">
    <!-- eliduprees-website-source head -->
    
    <link rel="stylesheet" type="text/css" href="/media/web-games/shared.css?rr">
    <link rel="stylesheet" href="/media/web-games/webfxr.css?rr">
    <link rel="stylesheet" href="/media/font-awesome-4.6.3/css/font-awesome.min.css?rr">
    
    <!-- /eliduprees-website-source head -->
</head>
<body>
    <script src="jquery-3.2.1.min.js"></script>
    
    <!-- eliduprees-website-source body -->
    <h1>WebFXR</h1>
    <p>Inspired by <a href="https://www.bfxr.net">Bfxr</a>. Generate sound effects for computer games. You have full rights to all sounds you make with WebFXR.</p>
    
    {app_element}
    
    <p>Notes for audio nerds:</p>
    <ul>
    <li>"Volume" is measured in decibels of amplitude above -40, because positive numbers are easier to work with. (Normally, 0.0 dB represents the maximum amplitude of 1.0, but here, 40.0 dB represents that.)</li>
    <li>Sine waves and square waves are normalized to have the same root-mean-square. A full-scale sine wave is 40.0 dB on the volume scale, while a full-scale square wave would actually be ~43 dB on this scale.</li>
    <li>When using harmonics, the Nth harmonic is given an amplitude of 1/N compared to the first harmonic.</li>
    <li>Fractional values of "harmonics" linearly attenuate the last harmonic, so that the effect is continuous. Values lower than 1.0 behave the same as 1.0.</li>
    <li>"Waveform skew" also functions as square duty. However, it goes through a logistic function first, so that you never run into the ends of the scale.</li>
    <li>The flanger doesn't have any feedback, it's just a sum of two copies of the signal with an offset. The input is called "frequency" – the reciprocal of the offset – so it can intuitively be on a log scale like the others.</li>
    <li>For the low-pass and high-pass filters, Bfxr used first-order digital RC filters. I always felt like the rolloff wasn't steep enough, so I chained 3 of them together, creating an amplitude rolloff of 30 dB per decade (equivalently, a power rolloff of 60 dB per decade).</li>
    <li>Bitcrush resolution reduction uses a novel formula for fractional bits to make the effect continuous. If it's between B bits and B+1 bits, it uses B+1 bits, but the rounding has a fractional bias towards even numbers. (Notice that a complete bias towards even numbers is the same as using one less bit.) I also tried a different method, where I used normal rounding and the possible sample values were 2^bits distance away from each other (using the fractional value of bits), but that didn't sound quite as continuous during a slide, despite being more elegant in some ways.</li>
    <li>The envelope doesn't <em>exactly</em> determine the length of the sound. Chorus, flanger, and bitcrush frequency can make the sound slightly longer, because the envelope is applied first, and those can make the sound linger.</li>
    </ul>
    <!-- /eliduprees-website-source body -->
    
    <!-- eliduprees-website-source after_body -->
    <script src="/media/web-games/lodash.js?rr"></script>
    <script src="/media/web-games/morphdom-umd.js?rr"></script>
    <script src="/media/web-games/shared-init.js?rr"></script>
    <script src="/media/web-games/webfxr-init.js?rr"></script>
    <script type="text/javascript" src="/media/audiobuffer-to-wav.js?rr"></script>
    <script type="text/javascript" src="/media/download.js?rr"></script>
    <!-- /eliduprees-website-source after_body -->
    
    <script async src="webfxr.js"></script>
</body>
</html>
"#, app_element=app_element)).unwrap();




  fs::write(static_path.join("media/web-games/webfxr.css"), &builder.css).unwrap();
}

