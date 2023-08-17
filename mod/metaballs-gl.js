const NUM_METABALLS = 50;

function prerenderBackground(size) {
    var canvas = document.getElementById("background");
    var gl = diva ? canvas.getContext("webgl", { preserveDrawingBuffer: true }) : canvas.getContext("webgl");
    if (diva) {
        canvas.width = divaW;
        canvas.height = divaH;
    }
    function compileShader(shaderSource, shaderType) {
        var shader = gl.createShader(shaderType);
        gl.shaderSource(shader, shaderSource);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            alert("YOUR MOM IS A POOP POOP");
        }
        return shader;
    }
    function getAttribLocation(program, name) {
        var attributeLocation = gl.getAttribLocation(program, name);
        if (attributeLocation === -1) {
            throw 'Can not find attribute ' + name + '.';
        }
        return attributeLocation;
    }
    function getUniformLocation(program, name) {
        var uniformLocation = gl.getUniformLocation(program, name);
        if (uniformLocation === -1) {
            throw 'Can not find uniform ' + name + '.';
        }
        return uniformLocation;
    }
    var vertex = compileShader(`
    attribute vec2 position;
    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
    }
    `, gl.VERTEX_SHADER);
    var fragment = compileShader(`
    precision highp float;
    uniform vec3 balls[` + NUM_METABALLS + `];
    uniform vec2 offset;
    uniform float yoopta;

    float w(float r, float x, float y) {
        return r / (abs(x) + abs(y));
    }

    void main(){
        const float BANDCOUNT = 4.0;
        float total = 0.0;
        for (int i = 0; i < ` + NUM_METABALLS + `; i ++) {
            float dx = balls[i].x + offset.x - gl_FragCoord.x;
            float dy = balls[i].y - yoopta + offset.y - yoopta - gl_FragCoord.y;
            float r = balls[i].z;
            total += (r * r) / (dx * dx + dy * dy);
        }
        gl_FragColor = vec4(floor(total * BANDCOUNT)/BANDCOUNT, 0.0, (1.0 - total) * 0.2, 1.0);
    }
    `, gl.FRAGMENT_SHADER);/*
    var error_log = gl.getShaderInfoLog(fragment);
    console.log(error_log);*/

    var program = gl.createProgram();
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.linkProgram(program);
    gl.useProgram(program);

    var texture = new Float32Array([ // shamelessly copypasted
        -1.0,  1.0, // top left
        -1.0, -1.0, // bottom left
        1.0,  1.0, // top right
        1.0, -1.0, // bottom right
    ]);

    var offset = new Float32Array([
        0.0, 0.0
    ]);

    var leBalls = [];
    for (var x = 0; x < NUM_METABALLS; x++) {
        leBalls.push({
            x: Math.random() * size,
            y: Math.random() * size,
            r: 50 + Math.random() * 150,
            xv: (Math.random() - 0.5),
            yv: (Math.random() - 0.5)
        });
    }
    var ballsHandle = getUniformLocation(program, 'balls');
    var offsetHandle = getUniformLocation(program, "offset");
    var yoopta = getUniformLocation(program, "yoopta");
    var toGPU = new Float32Array(3 * NUM_METABALLS);

    function updateBalls() {
        leBalls.forEach((ball, index) => {
            toGPU[index * 3] = ball.x;
            toGPU[index * 3 + 1] = ball.y;
            toGPU[index * 3 + 2] = ball.r;
        });
        gl.uniform3fv(ballsHandle, toGPU);
    }

    var tick = 0;

    function main(x, y) {
        gl.viewport(0, 0, window.innerWidth, window.innerHeight);
        var leWidth = window.innerWidth;
        var leHeight = window.innerHeight;
        if (diva) {
            leWidth = divaW;
            leHeight = divaH;
        }
        tick++;
        offset[0] = -x;
        offset[1] = y;
        gl.uniform2fv(offsetHandle, offset);
        gl.uniform1f(yoopta, size/2 - leWidth/2);
        leBalls.forEach(ball => {
            ball.x += ball.xv;
            ball.y += ball.yv;
            if (ball.x > size + leWidth/2) {
                ball.x = size + leWidth/2;
                ball.xv *= -1;
            }
            if (ball.x < -leWidth/2) {
                ball.x = -leWidth/2;
                ball.xv *= -1;
            }
            if (ball.y > size + leHeight/2) {
                ball.y = size + leHeight/2;
                ball.yv *= -1;
            }
            if (ball.y < -leHeight/2) {
                ball.y = -leHeight/2;
                ball.yv *= -1;
            }
        });
        updateBalls();

        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }

    var textureBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, textureBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, texture, gl.STATIC_DRAW);

    var positionHandle = getAttribLocation(program, 'position');
    gl.enableVertexAttribArray(positionHandle);
    gl.vertexAttribPointer(positionHandle, 2, gl.FLOAT, gl.FALSE, 2 * 4, 0);
    if (diva) {
        var divaMain = () => {
            main(0, 0);
            requestAnimationFrame(divaMain);
        };
        divaMain();
    }
    else {
        return main;
    }
}