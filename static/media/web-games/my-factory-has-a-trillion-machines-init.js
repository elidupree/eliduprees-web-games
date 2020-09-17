"use strict";

window.leaflet = L;

window.Module = window.Module || {};
//Module.canvas = document.getElementById ("canvas");
Module.TOTAL_STACK = 128*1024*1024;
Module.TOTAL_MEMORY = 256*1024*1024;

window.devicePixelRatio = window.devicePixelRatio || 1.0;

window.context = document.getElementById("canvas").getContext('2d');

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
  return [x,y,offset.width,offset.height];
};
window.mouse_callback = function (callback) {
  return function(event) {
    var xywh = mouse_coords(event);
    (callback)(xywh[0],xywh[1],xywh[2],xywh[3]);
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

