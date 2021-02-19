"use strict";

import init, { MouseCssPositionOnMap, ClickType, rust_init, do_frame, rust_mousedown, rust_mousemove, rust_mouseup, }
  from '/my-factory-has-a-trillion-machines/pkg/my_factory_has_a_trillion_machines.js';

async function run() {
  await init();

const leaflet = L;
const canvas = document.getElementById("canvas");
const context = canvas.getContext('2d');

var roundUpToPowerOfTwo = (n) => {
  console.assert(n >= 1 && n <= (1 << 30), "roundUpToPowerOfTwo range error");
  return 1 << (32 - Math.clz32(n - 1));
};

var images = [
  ["chest", "/media/web-games/factory-images/chest.png?rr"],
  ["conveyor", "/media/web-games/factory-images/conveyor.png?rr"],
  ["iron", "/media/web-games/factory-images/iron.png?rr"],
  ["machine", "/media/web-games/factory-images/machine.png?rr"],
  ["merger", "/media/web-games/factory-images/merger.png?rr"],
  ["mine", "/media/web-games/factory-images/mine.png?rr"],
  ["ore", "/media/web-games/factory-images/ore.png?rr"],
  ["splitter", "/media/web-games/factory-images/splitter.png?rr"],
  ["rounded-rectangle-solid", "/media/web-games/factory-images/rounded-rectangle-solid.png?rr"],
  ["rounded-rectangle-transparent", "/media/web-games/factory-images/rounded-rectangle-transparent.png?rr"],
  ["input", "/media/web-games/factory-images/input.png?rr"]
];

window.mouse_coords = function (event) {
  var offset = canvas.getBoundingClientRect();
  var x = (event.clientX - offset.left);
  var y = offset.height - (event.clientY - offset.top);
  return new MouseCssPositionOnMap({
    x, y,
    width: offset.width,
    height: offset.height,
  })
};
window.mouse_callback = function (callback) {
  return function(event) {
    (callback)(mouse_coords(event));
  }
};

Promise.all(images.map((image) =>
  new Promise((resolve, reject) => {
    var img = new Image();
    img.addEventListener('load', () => resolve([image [0], img]), false);
    img.addEventListener('error', () => reject([image [0], img]), false);
    img.src = image [1];
}))).then(
  (images) => {
    window.loaded_sprites = {}
    for(var img of images) {
      window.loaded_sprites[img[0]] = img[1]
    }
  },
  (img) => {
    console.log("image loading failed: ", img[1].src);
  });
/*
  (images) => {
    var coords = {};
    var coord = 0;
    var totalheight = 1;
    var maxwidth = 1;
    for(var img of images) {
      var width = Math.min (64, img [1].width);
      var height = Math.min (64, img [1].height);
      coords[img [0]] = {
        x: 2,
        y: totalheight,
        width,
        height 
      };
      totalheight += height + 2;
      if(maxwidth < width) { maxwidth = width; }
    }
    var textureheight = roundUpToPowerOfTwo(totalheight);
    var texturewidth = roundUpToPowerOfTwo(maxwidth + 4);
    var canvas = document.createElement('canvas');
    canvas.width = texturewidth;
    canvas.height = textureheight;
    var ctx = canvas.getContext('2d');
    // Wacky color to make mistakes using this texture obvious;
    // to make them unobtrusive instead, use 'rgba(255, 255, 255, 0)'
    ctx.fillStyle = 'rgba(255, 0, 127, 0.5)';
    ctx.fillRect(0, 0, texturewidth, textureheight);
    for(var img of images) {
      var c = coords[img[0]];
      // drawImage() doesn't copy the alpha of the image rectangle
      // (unless globalCompositeOperation = 'copy' but then that
      // erases the rest of the destination image)
      // so use putImageData with the ImageData we extract here.
      var subcanvas = document.createElement('canvas');
      subcanvas.width = c.width;
      subcanvas.height = c.height;
      var subctx = subcanvas.getContext('2d');
      // hopefully 'copy' preserves even the rgb values of alpha=0 pixels,
      // because those might matter to some antialiasing approaches
      subctx.globalCompositeOperation = 'copy';
      subctx.drawImage(img[1], 0, 0, c.width, c.height);
      ctx.putImageData(subctx.getImageData(0, 0, c.width, c.height), c.x, c.y);
    }
    var imageData = ctx.getImageData(0, 0, texturewidth, textureheight);
    return { rgba: imageData.data, width: texturewidth, height: textureheight, coords: coords };
  },
  (img) => {
    console.log("image loading failed: ", img[1].src);
  }).then((textureinfo) => {
    window.loaded_sprites = textureinfo;
    console.log(textureinfo);
    var canvas = document.createElement('canvas');
    canvas.style = "background-color: blue";
    canvas.width = textureinfo.width;
    canvas.height = textureinfo.height;
    canvas.getContext('2d').putImageData(new ImageData(textureinfo.rgba, textureinfo.width, textureinfo.height), 0, 0);
    document.getElementsByTagName('body')[0].appendChild(canvas);
  });*/
  
  
  
window.leaflet_map = leaflet.map ('leaflet_map', {
  crs: leaflet.CRS.Simple,
  minZoom: -5,
  //zoomAnimation: false,
  center: [0, 0],
  zoom: 5,
});

leaflet.marker ([0, 0]).addTo (leaflet_map).bindPopup ("thingy thingy").openPopup();


const app_element = document.getElementById("app");
const inventory_element = document.getElementById("inventory");


function mousedown_callback(event) {
  (rust_mousedown)(mouse_coords(event), new ClickType({buttons: event.buttons, shift: event.shiftKey, ctrl: event.ctrlKey}));
};

var dpr = window.devicePixelRatio || 1.0;
var width = 800;
var height = 800;
var physical_width = height*dpr;
var physical_height = width*dpr;
canvas.style.width = width+"px";
canvas.style.height = height+"px";
canvas.width = physical_width;
canvas.height = physical_height;
leaflet_map.on("mousedown", function(event) { mousedown_callback(event.originalEvent); });
//window.leaflet_map.on("contextmenu", function(e) {e.preventDefault()});
document.body.addEventListener("mouseup", mouse_callback (rust_mouseup));
document.body.addEventListener("mousemove", mouse_callback (rust_mousemove));


window.init_machine_type = function (name) {
  const id = `machine_choice_${name}`;
  const radio = document.createElement("input");
  radio.type = "radio";
  radio.id = id;
  radio.name = "machine_choice";
  radio.value = name;
  radio.checked = (name === "Iron mine");
  const label = document.createElement("label");
  label.setAttribute("for", id);
  label.textContent = name;
  let onclick;
  if (name === "Conveyor") {
    onclick = () => leaflet_map.dragging.disable();
  } else {
    onclick = () => leaflet_map.dragging.enable();
  }
  radio.addEventListener("click", onclick);
  app_element.appendChild(radio);
  app_element.appendChild(label);
    console.log(clear_canvas);
};

window.gather_dom_samples = function () {
  var map_zoom = leaflet_map.getZoom();
  var offset = canvas.getBoundingClientRect();
  return {
    map_zoom: map_zoom,
    map_css_scale: leaflet_map.getZoomScale(leaflet_map.getZoom(), 0),
    map_world_center: [leaflet_map.getCenter().lng, leaflet_map.getCenter().lat],
    canvas_backing_size: [context.canvas.width, context.canvas.height],
    canvas_css_size: [offset.width, offset.height],
    device_pixel_ratio: window.devicePixelRatio,
    current_mode: document.querySelector("input[name=machine_choice]:checked").value,
  };
};

window.clear_canvas = function () {
  context.fillStyle = "white";
  context.fillRect(0, 0, context.canvas.width, context.canvas.height);
};

window.draw_sprite = function (
  sprite, cx, cy, sx, sy, quarter_turns_from_posx_towards_posy,
) {
  context.save();
  //context.scale(context.canvas.width, context.canvas.height);
  context.translate (cx, cy);
  context.rotate (-(Math.PI*0.5) * quarter_turns_from_posx_towards_posy);

  var sprite = loaded_sprites[sprite];

  context.drawImage (sprite, -sx/2.0,-sy/2.0, sx,sy);
  /*context.globalCompositeOperation = "lighter";
  var r = @{color[0]*255.0};
  var g = @{color[1]*255.0};
  var b = @{color[2]*255.0};
  context.fillStyle = "rgb("+r+","+g+","+b+")";
  context.fillRect (@{corner[0]},@{corner[1]}, @{size [0]},@{size [1]});*/

  context.restore();
};

window.update_inventory = function (inventory) {
  for (const [material, amount] of Object.entries(inventory)) {
    const id = `inventory_${material}`;
    let element = document.getElementById(id);
    if (element === null) {
      element = document.createElement("div");
      element.id = id;
      inventory_element.appendChild(element);
    }
    element.textContent = `${material}: ${amount}`;
  }
};


  rust_init();

  function frame() {
    window.requestAnimationFrame(frame);
    if (window.loaded_sprites === undefined) {
      return;
    }
    do_frame();
  }
  frame();
}

run();
