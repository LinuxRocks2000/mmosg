## MMOSG Map Viewer by Tyler Clarke
## This literally just creates a PNG file to represent map json

import json
from PIL import Image, ImageDraw, ImageFont

infile = input("map filename: ")
outfile = infile + ".png"
world = json.loads(open(infile).read())
image = Image.new(mode="RGB", size=(1000,1000), color=(255, 255, 255)) ## always 1000x1000. Shrink the map to fit.
ctx = ImageDraw.Draw(image)
font = ImageFont.load_default()
ctx.text((0, 0), "mmosg map visualizer: " + infile + " at " + str(world["world_size"]) + "px", font=font, align="left", fill="black")
ctx.text((1000 - font.getlength("by tyler clarke"), 0), "by tyler clarke", font=font, align="left", fill="black")
scaledown = 1000/world["world_size"]
for element in world["map"]:
    x = element["x"] * scaledown
    y = element["y"] * scaledown
    w = element["w"] * scaledown
    h = element["h"] * scaledown
    ctx.rectangle([(x - w/2, y - h/2), (x + w/2, y + h/2)], fill="grey")
image.save(outfile)
image.show()
