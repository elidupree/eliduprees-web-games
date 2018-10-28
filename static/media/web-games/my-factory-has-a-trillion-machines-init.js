"use strict";


window.Module = window.Module || {};
Module.canvas = document.getElementById ("canvas");



var roundUpToPowerOfTwo = (n) => {
  console.assert(n >= 1 && n <= (1 << 30), "roundUpToPowerOfTwo range error");
  return 1 << (32 - Math.clz32(n - 1));
};

var images = ["/media/web-games/factory-images/chest.png?rr", "/media/web-games/factory-images/conveyor.png?rr", "/media/web-games/factory-images/iron.png?rr", "/media/web-games/factory-images/machine.png?rr", "/media/web-games/factory-images/merger.png?rr", "/media/web-games/factory-images/mine.png?rr", "/media/web-games/factory-images/ore.png?rr", "/media/web-games/factory-images/splitter.png?rr"]; 

Promise.all(images.map((path) =>
  new Promise((resolve, reject) => {
    var img = new Image();
    img.addEventListener('load', () => resolve(img), false);
    img.addEventListener('error', () => reject(img), false);
    img.src = path;
}))).then(
  (images) => {
    var coords = {};
    var coord = 0;
    var totalheight = 1;
    var maxwidth = 1;
    for(var img of images) {
      coords[img.src] = {
        x: 2,
        y: totalheight,
        width: img.width,
        height: img.height
      };
      totalheight += img.height + 2;
      if(maxwidth < img.width) { maxwidth = img.width; }
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
      var c = coords[img.src];
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
      subctx.drawImage(img, 0, 0, c.width, c.height);
      ctx.putImageData(subctx.getImageData(0, 0, c.width, c.height), c.x, c.y);
    }
    var imageData = ctx.getImageData(0, 0, texturewidth, textureheight);
    return { rgba: imageData.data, width: texturewidth, height: textureheight, coords: coords };
  },
  (img) => {
    console.log("image loading failed: ", img.src);
  }).then((textureinfo) => {
    console.log(textureinfo);
    var canvas = document.createElement('canvas');
    canvas.width = textureinfo.width;
    canvas.height = textureinfo.height;
    canvas.getContext('2d').putImageData(new ImageData(textureinfo.rgba, textureinfo.width, textureinfo.height), 0, 0);
    document.getElementsByTagName('body')[0].appendChild(canvas);
  });
  
