#!/usr/bin/python3
# -*- coding: utf-8 -*-

import os.path
import sys

web_games_path = sys.argv[1]
file = open (os.path.join (web_games_path, "static/media/web-games/webfxr.css"), "w")

name_column = 1
dice_column = name_column + 1
lock_column = dice_column + 1
toggle_column = lock_column + 1
input_column = toggle_column + 1
add_effect_column = input_column + 1
canvas_column = add_effect_column + 1

file.write ('''
html,body {background-color: white;}
p.disclaimer {font-size: 200%; text-align: center; margin:0.4em 0.8em;}
/*
#panels {display: flex; justify-content: center; flex-wrap: wrap; }
*/

#app {display: flex;}
.left_column {}

.main_grid {display: grid; grid-template-columns: 1fr 0fr 0fr 0fr 0fr 0fr 0fr 1fr;}
.signal_toggle {grid-column:'''+ str(toggle_column) +'''/ span 1; }
.labeled_input.numeric, .effect_header, .odd_harmonics_toggle, .view_toggle {grid-column: '''+ str(input_column) +''' / span 1; white-space: nowrap;}
.sound_radio_input {grid-column: '''+ str(input_column) +''' / span 2; }
.add_effect_buttons {grid-column: '''+ str(add_effect_column) +''' / span 1; width: 8em;}
.signal_effect {grid-column: '''+ str(input_column) +''' / span 2 !important; }
.toplevel_input_label {grid-column: '''+ str(name_column) +''' / span 1; text-align: right; align-self: center;}
.input_region {border: 0.125em solid #ccc; border-width: 0.0625em 0; grid-column: '''+ str(name_column) +''' / -1; }
.input_region:nth-child(odd) {background-color:#eee; }
.main_grid .canvas_wrapper {grid-column: '''+ str(canvas_column) +''' / span 1; align-self: center; }
.panel {margin:0.8em; padding:0.8em; background-color:#eee;}
.panel .labeled_input {margin:0.2em;}
.panel label {margin-left: 0.2em; margin-right: 0.6em;}
input[type='checkbox'] {width:2em;height:2em;}
input[type='radio'] {width:2em;height:2em;}
input[type='button'] {padding: 0 0.8em;}
input[type='number'] {width:5em;}
input,select {height:2em; vertical-align: middle; align-self: center;}
label {vertical-align: middle; align-self: center;}
input[type='button'].down {
  background-color: #bbb;
  background-image: linear-gradient(to top,#ddd,#999);
  border-color:#888 #777 #555 #888;
}
.signal_not_applicable {grid-column: '''+ str(input_column) +''' / span '''+ str(canvas_column + 1 - input_column) +'''; white-space: nowrap; font-style: italic; color: #555; padding:0.25em; }

@media screen and (max-width: 30em) {
  p.disclaimer {font-size: 85%;}
  #panels { }
}
''')

file.close()
