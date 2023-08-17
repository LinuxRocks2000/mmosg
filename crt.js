// Zachstronaut's CRT


function fakeCRT(source) {
    var glcanvas, source, texture, w, h, hw, hh, w75;
    try {
        glcanvas = fx.canvas();
    } catch (e) { return; }
    
    // This tells glfx what to use as a source image
    texture = glcanvas.texture(source);
    
    // Just setting up some details to tweak the bulgePinch effect
    w = source.width;
    h = source.height;
    hw = w / 2;
    hh = h / 2;
    w75 = w * 0.75;

    // Hide the source 2D canvas and put the WebGL Canvas in its place
    source.parentNode.insertBefore(glcanvas, source);
    source.style.display = 'none';
    glcanvas.className = source.className;
    glcanvas.id = source.id;
    glcanvas.style.display = source.style.display;
    source.id = 'old_' + source.id;

    return () => {
        // Load the latest source frame
        texture.loadContentsOf(source);
        
        // Apply WebGL magic
        glcanvas.draw(texture)
            .bulgePinch(glcanvas.width/2, glcanvas.height/2, glcanvas.width * 0.75, 0.12)
            .vignette(0.25, 0.3)
            .update();
    };
}