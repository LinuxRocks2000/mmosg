// This is an alternative way of doing fancy topography maps - using metaballs. I would consider it more promising.

function prerenderBackground(size) {
    var canvas = document.getElementById("background");
    canvas.width = size;
    canvas.height = size;
    var obj = {
        canvas: canvas,
        balls: [],
        breakpoints: [0, 0.05, 0.1, 0.2, 0.3, 0.5, 0.9, 1.5],
        ctx: canvas.getContext("2d"),
        intensityOf(x, y) {
            var ret = 0;
            this.balls.forEach(point => {
                var dx = point[0] - x;
                var dy = point[1] - y;
                ret += point[2] / (dx * dx + dy * dy);
            });
            return ret;
        },
        drawChunk(x, y, w, h) {
            const data = this.ctx.getImageData(x, y, w, h);
            const buffah = data.data;
            for (var _x = 0; _x < w; _x ++) {
                for (var _y = 0; _y < h; _y ++) {
                    var i = (_y * w + _x) * 4;
                    var intensity = this.intensityOf(x + _x, y + _y);
                    var insidePoint = 0;
                    this.breakpoints.forEach(point => {
                        //point = 1 / point;
                        if (intensity > point) {
                            insidePoint = point;
                        }
                    });
                    //if (isLegit) {
                    buffah[i] = clamp(0, Math.round(255 * insidePoint / this.breakpoints[this.breakpoints.length - 1]), 255); // assumes they're sorted in least-to-greatest order
                    //}
                    buffah[i + 3] = 255; // alpha channel
                }
            }
            this.ctx.putImageData(data, x, y);
        },
        tiles(x, y, w, h, size, time = 10) {
            for (let _x = x; _x < x + w; _x++) {
                for (let _y = y; _y < y + h; _y++) {
                    //setTimeout(() => {
                        this.drawChunk(_x * size, _y * size, size, size);
                    //}, 1/*(y * w + x) * time*/);
                }
            }
        }
    }
    for (var i = 0; i < 20; i++) {
        obj.balls.push([Math.random() * size, Math.random() * size, Math.random() * 100000]);
    }
    obj.tiles(0, 0, size/1000, size/1000, 1000);
    //obj.drawChunk(0, 0, 1000, 1000);
    //return obj;
}