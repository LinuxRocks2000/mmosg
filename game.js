var effect = "none";
const PI = Math.PI;
function coterminal(angle, about = PI * 2){
    while (angle < 0) {
        angle += about;
    }
    while (angle >= about) {
        angle -= about;
    }
    return angle;
}

function errorify(one, two, about = PI * 2) {
    one = coterminal(one, about);
    two = coterminal(two, about);
    var error = coterminal(two - one, about);
    if (error < PI) {
        error = error - PI * 2;
    }
    return error;
}

var canvas = document.getElementById("game");
var ctx = canvas.getContext("2d");
if (effect == "CRT") {
    var drawCRT = fakeCRT(canvas, ctx);
}

//ctx.oldDrawImage = ctx.drawImage;
//ctx.drawImage = function (image, x, y, w, h) {
    //x = Math.round(x);
    //y = Math.round(y);
    //w = Math.round(w);
    //h = Math.round(h);
    //this.oldDrawImage(image, x, y, w, h);
//};

ctx.makeRoundRect = function (x, y, width, height, rx, ry) { // Stolen from #platformer
    this.translate(x, y);
    this.moveTo(rx, 0);
    this.lineTo(width - rx, 0);
    this.quadraticCurveTo(width, 0, width, ry);
    this.lineTo(width, height - ry);
    this.quadraticCurveTo(width, height, width - rx, height);
    this.lineTo(rx, height);
    this.quadraticCurveTo(0, height, 0, height - ry);
    this.lineTo(0, ry);
    this.quadraticCurveTo(0, 0, rx, 0);
    this.translate(-x, -y);
};
if (ctx.roundRect == undefined){
    ctx.roundRect = function(x, y, width, height, radii) {
        ctx.makeRoundRect(x, y, width, height, radii, radii);
    };
}
var ping = false;
var connected = true;
var connection = connection = new WebSocket("ws" + (window.location.protocol == "https:" ? "s" : "") + "://" + location.host + location.pathname + (location.pathname.endsWith("/") ? "" : "/") + "game"); // Make it adaptive, so it fits in my reverse proxy and anywhere else
setInterval(() => {
    connected = !ping;
    connection.send("_");
    ping = true;
    var bannerNotConnected = document.getElementById("notConnected");
    if (!connected && !didSelectHome) {
        bannerNotConnected.style.display = "";
    }
    else {
        bannerNotConnected.style.display = "none";
    }
}, 500);
const DEBUG = false;
var playing = false;
var cX = 0;
var cY = 0;
var xv = 0;
var yv = 0;
var drawCallsThisTick = 0;
var gameSize = 0;
var keysDown = {};
var didSelectHome = false;
var isStratChange = false;
var counter = 0;
const upgrades = {
    menuActive: false,
    selectedMenu: undefined, // the word the menu is writing for
    "h": [
        {
            name: "ANTIAIRCRAFT",
            word: "a",
            cost: 10
        }
    ]
};
var hovered = undefined;
var selected = undefined;
var angleAdjustMode = false;
var score = 0;
var wallsRemaining = 4;
var wasPlayMode = false;
var myCastle = undefined;
var teamLeader = false;
function isCastle(value){
    return value == "c" || value == "R";
}
var canPlaceWall = false;
var inventoryMode = false;
var interpolator = 0;
var inventoryHovered = -1;
var wallsTurn = 2;
var banners = {};
var teams = {};
var inventoryPlace = undefined;
var timeToStart = -1;
var canPlaceCastle = false;
var sendNotifs = false;
var isFirstTick = true;
var CRTEffectWipe = 0;
var nearestCastleAngle = 0;
var totalTicksPassed = 0;
var startTime = 0;
var lastTick = 0;
var remainingHundredthsPretty = 0;
var remainingSecsPretty = 0;
var remainingMins = 0;
var sidebarScroll = 0;
var radDisplay = 0;
var jitter = 0;
var isRTF = false;
var me = "n";
var arrangment = "";
var timePerTick = 1000/30; // 30 FPS
var cantPlace = false;
var exploder = {
    x: 0,
    y: 0,
    time: 0
};
var message = {
    endTime: 0,
    error: false,
    content: "IF YOU CAN READ THIS YOU\nNEED TO TOUCH GRASS",
    sticky: false
};
const preloads = {
    basic_fighter: [
        document.getElementById("basic_fighter_img_mine"),
        document.getElementById("basic_fighter_img_enemy")
    ],
    sniper: [
        document.getElementById("sniper_img_mine"),
        document.getElementById("sniper_img_enemy")
    ],
    missile: [
        document.getElementById("missile_img_mine"),
        document.getElementById("missile_img_enemy")
    ],
    tie_fighter: [
        document.getElementById("tie_fighter_img_mine"),
        document.getElementById("tie_fighter_img_enemy")
    ],
    turret: [
        document.getElementById("turret_img_mine"),
        document.getElementById("turret_img_enemy")
    ],
    fort: [
        document.getElementById("fort_img_mine"),
        document.getElementById("fort_img_enemy")
    ],
    nuke: [
        document.getElementById("nuke_img_mine"),
        document.getElementById("nuke_img_mine")
    ],
    wall: document.getElementById("wall_img"),
    friendlyWall: document.getElementById("friendly_wall_img"),
    connection: [
        document.getElementById("connected"),
        document.getElementById("not_connected")
    ],
    message: {
        error: document.getElementById("error"),
        info: document.getElementById("info")
    },
    background: document.getElementById("preload_terrain")
};
var palette = {
    GREEN: "#00FF00",
    RED: "#FF3131",
    BLACK: "#000000",
    WHITE: "#FFFFFF",
    DARKGREEN: "#263435",
    MIDGREEN: "#327963",
    LIGHTGREEN: "#ACCF74",
    ORANGECREAM: "#D8965A",
    LIGHTBLUE: "#24E4D5",
    AZKABANGREEN: "#1F1E23"
};
const art = {
    f (cestDeMoi){
        ctx.rotate(Math.PI/2);
        ctx.drawImage(cestDeMoi ? preloads.basic_fighter[1] : preloads.basic_fighter[0], -30, -40);
    },
    c (){
        ctx.lineWidth = 4;
        ctx.beginPath();
        ctx.roundRect(-40, -40, 80, 80, 12);
        ctx.stroke();
    },
    w () {
        ctx.drawImage(preloads.wall, -25, -25, 50, 50);
    },
    W () {
        ctx.drawImage(preloads.friendlyWall, -40, -40);
    },
    s (cestDeMoi) {
        ctx.rotate(-Math.PI / 2);
        var scaleDown = 2/3;
        ctx.drawImage(cestDeMoi ? preloads.sniper[1] : preloads.sniper[0], -14, -40);
    },
    t (cestDeMoi) {
        ctx.drawImage(cestDeMoi ? preloads.tie_fighter[1] : preloads.tie_fighter[0], -31.5, -40);
    },
    T (cestDeMoi) {
        ctx.rotate(Math.PI / 2);
        ctx.drawImage(cestDeMoi ? preloads.turret[1] : preloads.turret[0], -37/2, -40);
    },
    h (cestDeMoi) {
        ctx.rotate(-Math.PI / 2);
        ctx.drawImage(cestDeMoi ? preloads.missile[1] : preloads.missile[0], -16, -40);
    },
    F (cestDeMoi) {
        ctx.rotate(Math.PI / 2);
        ctx.drawImage(cestDeMoi ? preloads.fort[1] : preloads.fort[0], -40, -40);
    },
    n(cestDeMoi) {
        ctx.rotate(Math.PI / 2);
        ctx.drawImage(cestDeMoi ? preloads.nuke[1] : preloads.nuke[0], -30, -30);
    },
    R(cestDeMoi) {
        ctx.rotate(-Math.PI / 2);
        ctx.lineWidth = 5;
        ctx.beginPath();
        ctx.moveTo(-50, 0);
        ctx.lineTo(50, 0);
        ctx.lineTo(25, -25);
        ctx.moveTo(50, 0);
        ctx.lineTo(25, 25);
        ctx.stroke();
    },
    draw(x, y, angle, thing, isMine = false, scaleDown = false, scaleFactor = window.innerWidth/2560){
        x = Math.round(x);
        if (!isMine){
            ctx.strokeStyle = palette.LIGHTGREEN;
        }
        else {
            ctx.strokeStyle = palette.ORANGECREAM;
        }
        ctx.fillStyle = ctx.strokeStyle;
        ctx.save();
        ctx.translate(x, y);
        ctx.rotate(angle);
        if (scaleDown){
            ctx.scale(scaleFactor, scaleFactor);
        }
        this[thing](isMine);
        ctx.restore();
        drawCallsThisTick++;
    },
    drawObj(object){
        if (DEBUG) {
            ctx.save();
            ctx.translate(object.x, object.y);
            ctx.rotate(object.angle);
            ctx.fillStyle = "yellow";
            ctx.globalAlpha = 0.3;
            ctx.fillRect(-object.w / 2, -object.h / 2, object.w, object.h);
            ctx.restore();
        }
        var cestDeMoi = mine.indexOf(object.id) == -1;
        if (myCastle && teams[myCastle.banner]) {
            cestDeMoi = cestDeMoi && teams[object.banner] != teams[myCastle.banner];
        }
        this.draw(object.x, object.y, object.angle, object.value, cestDeMoi, true, 0.6);
    }
};
document.getElementById("enableNotifs").onclick = () => {
    Notification.requestPermission().then((permission) => {
        if (permission === "granted") {
            sendNotifs = true;
        }
    });
    document.getElementById("enableNotifs").style.display = "none";
};
if (Notification.permission === "granted") {
    sendNotifs = true;
}
else {
    document.getElementById("enableNotifs").style.display = "";
}

function styleInfotextMajor(){
    ctx.fillStyle = palette.RED;
    ctx.textAlign = "center";
    ctx.font = "bold 40px monospace";
}

function makeProtocolMessage(command, args) {
    const OLDMODE = false;
    var ret = command;
    args.forEach((item, i) => {
        item = item + ""; // Make sure it's a string so we can send numbers safely
        if (OLDMODE){
            ret += item;
            if (i < args.length - 1){
                ret += " ";
            }
        }
        else{
            ret += String.fromCharCode(item.length);
            ret += item;
        }
    });
    return ret;
}

function sendProtocolMessage(command, args){
    connection.send(makeProtocolMessage(command, args));
}

function upgrade(id, upgrade) {
    sendProtocolMessage("U", [id, upgrade]);
}

function rtfUpgrade(thing) {
    if (isRTF) {
        upgrade(myCastle.id, thing);
    }
}

function readProtocolMessage(message) {
    var ret = {
        command: message[0],
        args: []
    };
    var i = 1;
    var buffer = "";
    while (i < message.length) {
        var iEnd = i + message.charCodeAt(i);
        while (i < iEnd) {
            i ++;
            buffer += message[i];
        }
        ret.args.push(buffer);
        buffer = "";
        i ++;
    }
    return ret;
}

function drawUpgradeMenu() {
    if (!upgrades.menuActive) {
        return;
    }
    const cappedLine = (x, y, w, h = 4 * scaleFactor) => {
        var capSize = Math.min(w, h);
        ctx.fillStyle = "#484D51";
        ctx.fillRect(x, y, w, h);
        ctx.fillStyle = "#9B9EA5";
        ctx.fillRect(x, y, capSize, capSize);
        ctx.fillRect(x + w - capSize, y + h - capSize, capSize, capSize);
    };
}

function drawCompass(compassRootX, compassRootY, size, range = 600){
    compassRootX += size/2;
    compassRootY += size/2;
    ctx.strokeStyle = palette.MIDGREEN;
    ctx.lineWidth = 4;
    ctx.beginPath();
    ctx.arc(compassRootX, compassRootY, size/2, 0, Math.PI * 2);
    ctx.fillStyle = palette.DARKGREEN;
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = "white";
    ctx.textAlign = "center";
    ctx.font = "12px monospace";
    ctx.fillText("90", compassRootX + size/2 + 20, compassRootY);
    ctx.fillText("0", compassRootX, compassRootY - size/2 - 20);
    ctx.fillText("180", compassRootX, compassRootY + size/2 + 20);
    ctx.fillText("270", compassRootX - size/2 - 20, compassRootY);
    ctx.fillStyle = palette.ORANGECREAM;
    ctx.translate(compassRootX, compassRootY);
    for (var i = 0; i < 12; i++) {
        var ang = (Math.PI / 6) * i;
        ctx.rotate(Math.PI / 6);
        if (i % 3 != 2) {
            ctx.beginPath();
            ctx.arc(size/2 + 10, 0, 2.5, 0, Math.PI * 2);
            ctx.fill();
        }
    }
    ctx.strokeStyle = palette.LIGHTGREEN;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.makeRoundRect(-12, -12, 24, 24, 6, 6);
    ctx.stroke();
    var ang = Math.atan2(myCastle.y - (cY + compassRootY), myCastle.x - (cX + compassRootX));
    ctx.rotate(ang);
    ctx.lineWidth = 10;
    ctx.strokeStyle = palette.ORANGECREAM;
    ctx.beginPath();
    ctx.arc(0, 0, size/2 - 5, -Math.PI / 24, Math.PI / 24);
    ctx.stroke();
    ctx.rotate(-ang);
    var closestCastle = undefined;
    var closestCastleDistance = Infinity;
    Object.values(objects).forEach(item => {
        var deltaX = item.x - myCastle.x;
        var deltaY = item.y - myCastle.y;
        var distance2 = deltaX * deltaX + deltaY * deltaY;
        if (item.value == "f" || item.value == "t" || item.value == "h") {
            if (mine.indexOf(item.id) == -1) {
                ctx.fillStyle = palette.ORANGECREAM;
            }
            else {
                ctx.fillStyle = palette.LIGHTGREEN;
            }
            if (distance2 < range * range) { // Squares are far less expensive than square roots
                deltaX *= (size/2 - 10) / range;
                deltaY *= (size/2 - 10) / range;
                ctx.beginPath();
                ctx.arc(deltaX, deltaY, 5, 0, Math.PI * 2);
                ctx.fill();
            }
        }
        else if (isCastle(item)) {
            deltaX = item.x - (cX + compassRootX);
            deltaY = item.y - (cY + compassRootY);
            distance2 = deltaX * deltaX + deltaY * deltaY;
            if (distance2 < closestCastleDistance) {
                closestCastleDistance = distance2;
                closestCastle = item;
            }
        }
    });
    if (closestCastle) {
        ang = Math.atan2(closestCastle.y - (cY + compassRootY), closestCastle.x - (cX + compassRootX));
        nearestCastleAngle = nearestCastleAngle * 0.9 + ang * 0.1;
        ctx.rotate(nearestCastleAngle);
        ctx.lineWidth = 5;
        ctx.strokeStyle = palette.ORANGECREAM;
        ctx.beginPath();
        ctx.arc(0, 0, size/2 - 15, -Math.PI / 35, Math.PI / 35);
        ctx.stroke();
        ctx.rotate(-nearestCastleAngle);
    }
    ctx.translate(-compassRootX, -compassRootY);
}

function drawMinimap(rootX, rootY, minimapSize){
    const scaling = minimapSize / gameSize;
    if (rootX < 0) {
        rootX = 0;
    }
    if (rootY < 0) {
        rootY = 0;
    }
    ctx.strokeStyle = palette.MIDGREEN;
    ctx.fillStyle = palette.DARKGREEN;
    ctx.lineWidth = 3;
    ctx.setLineDash([40, 10]);
    ctx.strokeRect(rootX, rootY, minimapSize, minimapSize);
    ctx.drawImage(preloads.background, rootX, rootY, minimapSize, minimapSize);//ctx.fillRect(rootX, rootY, minimapSize, minimapSize);
    ctx.setLineDash([]);
    ctx.beginPath();
    const capSize = 50;
    ctx.lineWidth = 5;
    ctx.strokeStyle = palette.LIGHTBLUE;
    ctx.moveTo(rootX + capSize, rootY);
    ctx.lineTo(rootX, rootY);
    ctx.lineTo(rootX, rootY + capSize);
    ctx.moveTo(rootX, rootY + minimapSize - capSize);
    ctx.lineTo(rootX, rootY + minimapSize);
    ctx.lineTo(rootX + capSize, rootY + minimapSize);
    ctx.moveTo(rootX + minimapSize - capSize, rootY + minimapSize);
    ctx.lineTo(rootX + minimapSize, rootY + minimapSize);
    ctx.lineTo(rootX + minimapSize, rootY + minimapSize - capSize);
    ctx.moveTo(rootX + minimapSize, rootY + capSize);
    ctx.lineTo(rootX + minimapSize, rootY);
    ctx.lineTo(rootX + minimapSize - capSize, rootY);
    ctx.stroke();
    var scaleDown = minimapSize / gameSize;
    ctx.fillStyle = palette.RED;
    ctx.fillRect(rootX + scaleDown * mousePos.gameX - 2.5, rootY + scaleDown * mousePos.gameY - 2.5, 5, 5);
    Object.values(objects).forEach(item => {
        var cestDeMoi = mine.indexOf(item.id) == -1;
        if (myCastle && teams[myCastle.banner]) {
            cestDeMoi = cestDeMoi && teams[item.banner] != teams[myCastle.banner];
        }
        if (cestDeMoi) {
            ctx.fillStyle = palette.ORANGECREAM;
            if (item.upgrades.indexOf("s") != -1) {
                return;
            }
        }
        else {
            ctx.fillStyle = palette.LIGHTGREEN;
        }
        ctx.strokeStyle = ctx.fillStyle;
        var convertedX = item.x * scaleDown;
        convertedX += rootX;
        var convertedY = item.y * scaleDown;
        convertedY += rootY;
        if (isCastle(item.value)) {
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.roundRect(convertedX - 5, convertedY - 5, 10, 10, 2);
            ctx.stroke();
            if (item.value == "R") {
                ctx.lineWidth = 2;
                ctx.beginPath();
                ctx.moveTo(convertedX, convertedY);
                ctx.lineTo(convertedX + Math.cos(item.angle - Math.PI / 2) * 10, convertedY + Math.sin(item.angle - Math.PI / 2) * 10);
                ctx.stroke();
            }
        }
        else if (item.value == "f" || item.value == "t" || item.value == "h" || item.value == "n") {
            ctx.beginPath();
            ctx.arc(convertedX, convertedY, 2, 0, Math.PI * 2);
            ctx.fill();
        }
        else if (item.value == "B") {
            ctx.rotate(item.angle);
            ctx.fillStyle = "grey";
            ctx.fillRect(convertedX - (item.w/2 * scaleDown), convertedY - (item.h/2 * scaleDown), item.w * scaleDown, item.h * scaleDown);
        }
        else if (!isNaN(item.value)) { // All NPCs are numbers 0-9
            ctx.fillStyle = "white";
            ctx.fillRect(convertedX - 1, convertedY - 1, 2, 2);
        }
    });
}

var inventoryStuff = [
    {
        name: "FIGHTER",
        word: "f",
        cost: 10,
        place: true
    },
    {
        name: "TIE",
        word: "t",
        cost: 20,
        place: true
    },
    {
        name: "SNIPER",
        word: "s",
        cost: 30,
        place: true
    },
    {
        name: "MISSILE",
        word: "h",
        cost: 5,
        place: true
    },
    {
        name: "TURRET",
        word: "T",
        cost: 100,
        place: true
    },
    {
        name: "FORT",
        word: "F",
        cost: 120,
        place: true
    },
    {
        name: "+2 WALL",
        word: "W",
        cbk: () => {
            if (score >= 30){
                wallsTurn += 2;
                wallsRemaining += 2;
                sendProtocolMessage("C", ["30"]);
            }
        },
        cost: 30
    },
    {
        name: "NUKE",
        word: "n",
        cost: 300,
        place: true
    }
];

var objects = {

};

var mine = [];

window.onkeydown = (evt) => {
    if (document.activeElement.id != "message") {
        keysDown[evt.key] = true;
    }
};

window.onkeyup = (evt) => {
    if (document.activeElement.id == "message") {
        if (evt.key == "Enter") {
            sendProtocolMessage('T', [document.activeElement.value]);
            document.activeElement.value = "";
        }
    }
    else {
        keysDown[evt.key] = false;
        if (evt.key == "i") {
            if (isStratChange && playing && didSelectHome){
                if (inventoryMode){
                    inventoryMode = false;
                }
                else{
                    inventoryMode = true;
                }
            }
            else{
                inventoryMode = false;
            }
        }
        else if (evt.key == "q"){
            if (wallsRemaining > 0){
                if (playing && isStratChange){
                    if (canPlaceWall){
                        wallsRemaining --;
                        sendProtocolMessage("p", ["w", mousePos.gameX, mousePos.gameY]);
                    }
                }
            }
        }
        else if (evt.key == "u") {
            if (upgrades[hovered]){
                upgrades.menuActive = !upgrades.menuActive;
                upgrades.selectedMenu = hovered;
            }
            else {
                upgrades.menuActive = false;
            }
        }
        else if (evt.key == "z" && evt.ctrlKey) {
            var el = document.getElementById("team-chat");
            if (el.style.display == "none") {
                el.style.display = "";
            }
            else {
                el.style.display = "none";
            }
            evt.preventDefault();
        }
    }
};

window.onwheel = (evt) => {
    if (inventoryMode){
        sidebarScroll += evt.deltaY;
    }
    else{
        if (keysDown["Shift"]) {
            xv += evt.deltaY;
            yv += evt.deltaX;
        }
        else {
            xv += evt.deltaX;
            yv += evt.deltaY;
        }
    }
};

function setName(name){
    document.title = name;
}

function zeroes(number, zeroesCount = 4) {
    var ret = "" + number;
    while (ret.length < zeroesCount){
        ret = "0" + ret;
    }
    return ret;
}

function splitString(string, delim = " "){
    var ret = [];
    var buf = "";
    var escape = false;
    for (var i = 0; i < string.length; i ++){
        if (!escape){
            if (string[i] == '\\'){
                escape = true;
                continue;
            }
            else if (string[i] == delim){
                ret.push(buf);
                buf = "";
            }
        }
        buf += string[i];
    }
    if (buf.length > 0){
        ret.push(buf);
    }
    return ret;
}

function drawRadiometer(scaleFactor){
    ctx.fillStyle = palette.AZKABANGREEN;
    var rootX = 1021 * scaleFactor;
    var rootY = window.innerHeight - 120 * scaleFactor;
    ctx.fillRect(rootX, rootY, 444 * scaleFactor, 85 * scaleFactor);
    var colors = [
        "#E6EFEA",
        "#9FADA5",
        "#708077",
        "#405449",
        "#283D31"
    ];
    colors.forEach((color, i) => {
        ctx.fillStyle = color;
        ctx.fillRect(1044 * scaleFactor + 82 * scaleFactor * i, window.innerHeight - 81 * scaleFactor, 70 * scaleFactor, 10 * scaleFactor);
        ctx.fillStyle = "#414E53";
        ctx.fillText(zeroes(i, 2), 1063 * scaleFactor + i * scaleFactor * 82, window.innerHeight - 87 * scaleFactor);
    });
    ctx.fillStyle = "#E6EFEA";
    var radiationUnderCursor = 0;
    Object.values(objects).forEach(item => {
        if (item.value == "r"){
            if (mousePos.gameX > item.x - item.w/2 && mousePos.gameX < item.x + item.w/2 && mousePos.gameY > item.y - item.h/2 && mousePos.gameY < item.y + item.h/2){
                radiationUnderCursor += item.radStrength;
            }
        }
    });
    radDisplay = radDisplay * 0.9 + radiationUnderCursor * 0.1;
    ctx.save();
    ctx.translate(1044 * scaleFactor + (444 - 23 * 2) * scaleFactor * radDisplay, window.innerHeight - 60 * scaleFactor);
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(-10, 15);
    ctx.lineTo(10, 15);
    ctx.fill();
    ctx.restore();
}

function drawSidebar(scaleFactor = window.innerWidth / 2560) {
    var sidebarY = mousePos.realY - sidebarScroll;
    ctx.strokeStyle = palette.AZKABANGREEN;
    ctx.lineWidth = 41 * scaleFactor * 2;
    ctx.strokeRect(0, 0, window.innerWidth, window.innerHeight);
    var rootWidth = window.innerWidth * 1 / 4.5;
    inventoryMode = mousePos.realX < rootWidth;
    ctx.fillStyle = palette.AZKABANGREEN;
    ctx.fillRect(0, 0, rootWidth, window.innerHeight);
    ctx.translate(0, sidebarScroll);
    var dasImg = preloads.connection[1];
    ctx.font = 70 * scaleFactor + "px Roboto";
    var words = "OFFLINE";
    if (connected) {
        dasImg = preloads.connection[0];
        words = "ONLINE";
    }
    var imgSize = rootWidth * 1 / 5;
    ctx.drawImage(dasImg, Math.round(rootWidth / 10), Math.round(rootWidth / 10), imgSize, imgSize);
    ctx.fillStyle = "#85A1AD";
    ctx.textAlign = "right";
    ctx.fillText(words, 495 * scaleFactor, rootWidth * 0.186);
    ctx.fillText((remainingMins < 10 ? "0" : "") + remainingMins + ":" + (remainingSecsPretty < 10 ? "0" : "") + remainingSecsPretty + ":" + (remainingHundredthsPretty < 10 ? "0" : "") + remainingHundredthsPretty, 495 * scaleFactor, 200 * scaleFactor);
    const cappedLine = (x, y, w, h = 4 * scaleFactor) => {
        var capSize = Math.min(w, h);
        ctx.fillStyle = "#484D51";
        ctx.fillRect(x, y, w, h);
        ctx.fillStyle = "#9B9EA5";
        ctx.fillRect(x, y, capSize, capSize);
        ctx.fillRect(x + w - capSize, y + h - capSize, capSize, capSize);
    };
    cappedLine(57 * scaleFactor, 33 * scaleFactor, 439 * scaleFactor);
    cappedLine(181 * scaleFactor, 122 * scaleFactor, 315 * scaleFactor);
    cappedLine(57 * scaleFactor, 215 * scaleFactor, 439 * scaleFactor);
    cappedLine(57 * scaleFactor, 281 * scaleFactor, 439 * scaleFactor);
    cappedLine(57 * scaleFactor, 319 * scaleFactor, 4 * scaleFactor, 466 * scaleFactor);
    cappedLine(492 * scaleFactor, 319 * scaleFactor, 4 * scaleFactor, 466 * scaleFactor);
    cappedLine(73 * scaleFactor, 317 * scaleFactor, 407 * scaleFactor);
    cappedLine(73 * scaleFactor, 779 * scaleFactor, 407 * scaleFactor);
    ctx.fillStyle = "#85A1AD";
    ctx.textAlign = "left";
    ctx.fillText("SHOP", 73 * scaleFactor, (215 + 60) * scaleFactor);
    ctx.beginPath();
    ctx.roundRect(314 * scaleFactor, 236 * scaleFactor, 99 * scaleFactor, 35 * scaleFactor, 4 * scaleFactor);
    ctx.fill();
    ctx.font = 30 * scaleFactor + "px Roboto";
    ctx.fillText(score, 440 * scaleFactor, (234 + 30) * scaleFactor);
    ctx.fillStyle = "#484D51";
    ctx.fillText("SCORE", 316 * scaleFactor, (234 + 30) * scaleFactor);

    // SHOP
    for (var x = 0; x < 3; x ++) {
        cappedLine(73 * scaleFactor, (443 + x * 103) * scaleFactor, 407 * scaleFactor);
    }
    inventoryStuff.forEach((item, i) => {
        if (item.stack == undefined) {
            item.stack = Infinity;
        }
        if (item.stack == 0) {
            ctx.globalAlpha = 0.3;
        }
        var x = 74 + (i % 2) * 203.5;
        var y = 340 + 103 * Math.floor(i / 2);
        x *= scaleFactor;
        y *= scaleFactor;
        if (item.word){
            art.draw(x + (50 - 20 * (1 - (i % 2))) * scaleFactor, y + 50 * scaleFactor, Math.PI/2, item.word, false, true);
        }
        ctx.fillStyle = "#FBFBD6";
        ctx.font = 32 * scaleFactor + "px Roboto";
        ctx.textAlign = "right";
        ctx.fillText(item.name, x + 200 * scaleFactor, y + (32) * scaleFactor);
        ctx.beginPath();
        ctx.roundRect(x + 128 * scaleFactor, y + 40 * scaleFactor, 72 * scaleFactor, 35 * scaleFactor, 4 * scaleFactor);
        ctx.fillStyle = "#85A1AD";
        ctx.fill();
        ctx.font = 30 * scaleFactor + "px Roboto";
        ctx.fillStyle = "#484D51";
        ctx.textAlign = "left";
        ctx.fillText(zeroes(item.cost), x + 130 * scaleFactor, y + 68 * scaleFactor);
        if (item.stack != 0) {
            if (didSelectHome && playing && (isStratChange || isRTF) && score >= item.cost){
                if (mousePos.realX > x && mousePos.realX < (x + 203.5 * scaleFactor) && sidebarY > y && sidebarY < (y + 103 * scaleFactor)) {
                    inventoryHovered = i;
                    ctx.strokeStyle = "yellow";
                    ctx.lineWidth = 2;
                    ctx.strokeRect(x, y, 203.5 * scaleFactor, 103 * scaleFactor);
                }
                if (inventoryPlace == i){
                    ctx.strokeStyle = "red";
                    ctx.lineWidth = 4;
                    ctx.strokeRect(x, y, 203.5 * scaleFactor, 103 * scaleFactor);
                }
            }
        }
        ctx.globalAlpha = 1;
    });

    if (keysDown["c"] && myCastle) {
        drawCompass(57 * scaleFactor, 827 * scaleFactor, 439 * scaleFactor);
    }
    else {
        drawMinimap(57 * scaleFactor, 807 * scaleFactor, 439 * scaleFactor);
    }

    ctx.translate(0, -sidebarScroll);
    drawRadiometer(scaleFactor);
}

function drawMessage(scaleFactor = window.innerWidth/2560){
    if (window.performance.now() < message.endTime || message.sticky) {
        var messageHeight = 91 * scaleFactor;
        var messageWidth = 621 * scaleFactor;
        var messageRootX = (window.innerWidth - messageWidth) - 10;
        var messageRootY = (window.innerHeight - messageHeight) - 10;
        ctx.fillStyle = palette.AZKABANGREEN;
        ctx.fillRect(messageRootX - 10 * scaleFactor, messageRootY - 10 * scaleFactor, messageWidth + 40 * scaleFactor, messageHeight + 40 * scaleFactor);
        if (message.error) {
            ctx.fillStyle = "#FF7559";
        }
        else {
            ctx.fillStyle = "#76C2D9";
        }
        ctx.fillRect(messageRootX, messageRootY, messageWidth, messageHeight);
        var art = preloads.message.error;
        if (message.error) {
            ctx.fillStyle = "#8C4031";
        }
        else {
            art = preloads.message.info;
            ctx.fillStyle = "#437282";
        }
        ctx.drawImage(art, messageRootX + 63 * scaleFactor, messageRootY + 5 * scaleFactor, messageHeight - 10 * scaleFactor, messageHeight - 10 * scaleFactor);
        ctx.font = 30 * scaleFactor + "px Inter";
        ctx.textAlign = "left";
        var textBaseline = messageRootY;
        message.content.split("\n").forEach((line) => {
            textBaseline += 36 * scaleFactor;
            ctx.fillText(line, messageRootX + 205 * scaleFactor, textBaseline);
        });
    }
}

function writeMessage(m) {
    var message = document.createElement("p");
    message.innerHTML = m;
    document.getElementById("team-chat").insertBefore(message, document.getElementById("message-outer"));
}

connection.onmessage = (message) => {
    //var data = message.data;
    //var command = data[0];
    //var args = data.substring(1, data.length).split(" ");
    var data = readProtocolMessage(message.data);
    command = data.command;
    args = data.args;
    if (command == "B"){
        writeMessage(args[0]);
    }
    else if (command == "s"){
        if (mode == 1) {
            playing = true;
            play();
        }
        console.log("an event was successful");
    }
    else if (command == "?") {
        teamLeader = true;
    }
    else if (command == "!"){
        if (timeToStart == -1) {
            if (!document.hasFocus() && sendNotifs){
                const notification = new Notification("Countdown started!");
            }
        }
        timeToStart = args[0];
    }
    else if (command == "e"){
        if (mode == 1 && args[0] == 0){
            alert("Wrong code!");
            window.location.reload();
        }
        else{
            console.error("Got ERROR " + args[0]);
        }
    }
    else if (command == "b"){
        banners[args[0]] = args[1];
        if (args.length > 2){
            console.log(data);
            teams[args[0]] = args[2]; // Team association: the just-set banner is a member of this team
        }
    }
    else if (command == "T"){
        alert("Game over! It was a tie - nobody was left alive at the end. Pretty L, in my opinion.");
    }
    else if (command == "W"){
        alert("You won! Your final score: " + score);
    }
    else if (command == "p") {
        document.getElementById("code-outer").style.display = "none";
    }
    else if (command == "E"){
        alert("Game over! <" + banners[args[0]] + "> won.");
    }
    else if (command == "w"){
        console.warn("Got warning " + args[0]);
        if (mode == 1){
            play();
        }
    }
    else if (command == "m"){
        gameSize = args[0] - 0;
        sRand = mulberry32(args[1] - 0);
        drawClarkeSquares(getTerrainContext(gameSize), clarkeSquares(gameSize));
    }
    else if (command == "n"){
        var ob = {
            value: args[0],
            id: args[1] - 0,
            x: args[2] - 0,
            y: args[3] - 0,
            angle: args[4] - 0,
            isEditable: args[5] == "1",
            goalPos: {
                x: args[2] - 0,
                y: args[3] - 0,
                angle: args[4] - 0
            },
            banner: args[6],
            radStrength: 0.3,
            w: args[7],
            h: args[8],
            upgrades: []
        };
        for (var i = 9; i < args.length; i ++) {
            console.log(args.length);
            ob.upgrades.push(args[i]);
        }
        ob.g_angle = ob.angle;
        ob.g_x = ob.x;
        ob.g_y = ob.y;
        ob.g_w = ob.w;
        ob.g_h = ob.h;
        ob.o_angle = ob.angle;
        ob.o_x = ob.x;
        ob.o_y = ob.y;
        ob.o_w = ob.w;
        ob.o_h = ob.h;
        objects[args[1] - 0] = ob;
    }
    else if (command == "r"){
        objects[args[0] - 0].radStrength = args[1] - 0;
    }
    else if (command == "a"){
        console.log(args);
        /*if (!myCastle){ // Your castle is always the first object you receive
            myCastle = objects[args[0] - 0];
            myCastle.health = 3;
        }*/
        mine.push(args[0] - 0);
    }
    else if (command == "t"){
        lastTick = window.performance.now();
        if (args.length == 3) {
            myCastle.health = args[2] - 0;
        }
        totalTicksPassed++;
        var timeElapsedSinceStart = window.performance.now() - startTime;
        var gTimePerTick = timeElapsedSinceStart / totalTicksPassed;
        timePerTick = timePerTick * 0.8 + gTimePerTick * 0.2; // Resist spikes
        counter = args[0] - 0;
        if (isFirstTick){
            startTime = window.performance.now();
            if (!document.hasFocus() && sendNotifs) {
                const notification = new Notification("Game started!");
            }
        }
        isFirstTick = false;
        if (args[1] == "1"){
            isStratChange = true;
            if (wasPlayMode){
                wallsRemaining = wallsTurn;
            }
            wasPlayMode = false;
        }
        else{
            isStratChange = false;
            wasPlayMode = true;
        }
        if (isRTF){
            sendProtocolMessage("R", [((keysDown["ArrowUp"] || keysDown["w"]) ? "1" : "0"), ((keysDown["ArrowLeft"] || keysDown["a"]) ? "1" : "0"), ((keysDown["ArrowRight"] || keysDown["d"]) ? "1" : "0"), ((keysDown["ArrowDown"] || keysDown["s"]) ? "1" : "0"), (keysDown[" "] ? "1" : "0")]);
        }
    }
    else if (command == "M"){
        var obj = objects[args[0] - 0];
        if (obj) {
            /*obj.o_x = obj.g_x;
            obj.o_y = obj.g_y;
            obj.o_angle = obj.g_angle;
            obj.o_w = obj.g_w;
            obj.o_h = obj.g_h;*/
            obj.o_x = obj.g_x;
            obj.o_y = obj.g_y;
            obj.o_angle = obj.g_angle;
            obj.o_w = obj.g_w;
            obj.o_h = obj.g_h;
            obj.g_x = args[1] - 0;
            obj.g_y = args[2] - 0;
            if (args.length > 3){
                obj.g_angle = args[3] - 0;
            }
            if (args.length > 5) {
                obj.g_w = args[4];
                obj.g_h = args[5];
            }
        }
    }
    else if (command == "d"){
        ctx.fillStyle = "red";
        var object = objects[args[0] - 0];
        if (object) {
            if (object.value == "n"){
                exploder.x = object.x;
                exploder.y = object.y;
                exploder.time = 5;
            }
            delete objects[args[0] - 0];
        }
    }
    else if (command == "u") {
        objects[args[0]].upgrades.push(args[1]);
    }
    else if (command == "S"){
        score = args[0] - 0;
    }
    else if (command == "l"){
        playing = false;
        mine = [];
        alert("You lose! You can close the tab now, or continue as a spectator. If it makes you feel any better, the person responsible for your loss got 50 points!");
    }
    else if (command == "_"){
        ping = false; // pong.
    }
    else if (command == "j"){
        jitter += args[0] - 0;
    }
    else {
        console.warn("Got unrecognizable data: ");
        console.log(data);
    }
};
var mode = 0;
function start(){
    me = document.getElementById("startingArrangment").value;
    if (me == "rtf"){
        isRTF = true;
    }
    if (me == "defender"){
        wallsTurn = 4;
    }
    if (isRTF) {
        inventoryStuff = [
            {
                name: "+GUN SPEED",
                cost: 30,
                stack: 1,
                cbk: () => {
                    sendProtocolMessage("C", [30]);
                    rtfUpgrade("b");
                }
            },
            {
                name: "SNIPER",
                cost: 40,
                stack: 1,
                cbk: () => {
                    sendProtocolMessage("C", [40]);
                    rtfUpgrade("s");
                }
            },
            {
                name: "SPEEDSHIP",
                cost: 70,
                stack: 1,
                cbk: () => {
                    sendProtocolMessage("C", [70]);
                    rtfUpgrade("f");
                }
            },
            {
                name: "FAST HEAL",
                cost: 150,
                stack: 1,
                cbk: () => {
                    sendProtocolMessage("C", [150]);
                    rtfUpgrade("h");
                }
            }
        ];
    }
    var banner = document.getElementById("banner").value.trim();
    if (banner == ""){
        banner = randomBanner();
    }
    var passcode = document.getElementById("code").value.trim();
    if (banner.length == 0){
        banner = "Unnamed Player";
    }
    sendProtocolMessage("c", [passcode, banner, me]);
    mode = 1; // waiting for a response
}

var mousePos = {
    gameX: 0,
    gameY: 0,
    realX: 0,
    realY: 0
};

document.getElementById("game").addEventListener("mousemove", (evt) => {
    mousePos.realX = evt.clientX;
    mousePos.realY = evt.clientY;
});

function mainloop(){
    if (!myCastle) {
        myCastle = objects[mine[0]];
        if (myCastle) {
            if (teams[myCastle.banner]) {
                document.getElementById("who").innerText = "all teammates";
                document.getElementById("who2").style.display = "";
                //document.getElementById("team-chat").style.display = ""; // UNCOMMENT THIS TO MAKE CHAT APPEAR TO ALL USERS
            }
        }
    }
    let timeSinceLastTick = window.performance.now() - lastTick;
    interpolator = timeSinceLastTick/timePerTick;
    if (interpolator > 1) {
        interpolator = 1;
    }
    Object.values(objects).forEach(object => {
        object.x = object.g_x * interpolator + object.o_x * (1 - interpolator);
        object.y = object.g_y * interpolator + object.o_y * (1 - interpolator);
        object.angle = object.g_angle * interpolator + object.o_angle * (1 - interpolator);
        object.w = object.g_w * interpolator + object.o_w * (1 - interpolator);
        object.h = object.g_h * interpolator + object.o_h * (1 - interpolator);
        if (interpolator == 1){
            object.o_w = object.g_w;
            object.o_h = object.g_h;
            object.o_angle = object.g_angle;
            object.o_x = object.g_x;
            object.o_y = object.g_y;
        }
    });
    if (isRTF && !isStratChange && counter != 0 && myCastle){
        cX = myCastle.x - window.innerWidth / 2;
        cY = myCastle.y - window.innerHeight / 2;
    }
    if (isRTF){
        wallsRemaining = 0; // you can't ever place walls as an RTF
    }
    message.sticky = false;
    if (playing) {
        if (!didSelectHome || (inventoryPlace != undefined && inventoryStuff[inventoryPlace].word == 'F')){
            cantPlace = !canPlaceCastle;
        }
        else if (isStratChange) {
            cantPlace = !canPlaceWall;
        }
        if (cantPlace){
            message.error = true;
            message.sticky = true;
            message.content = "CAN'T PLACE HERE!";
        }
        else {
            if (isStratChange){
                message.error = false;
                message.sticky = true;
                if (inventoryPlace){
                    message.content = "PLACE YOUR NEW\nGAMEPIECE";
                }
                else{
                    message.content = "MOVE YOUR SHIPS " + (teamLeader ? "YOU ARE TL" : "");
                    if (wallsRemaining > 0){
                        message.content += "\nWALLS REMAINING: " + wallsRemaining;
                    }
                }
            }
            else if (counter == 0){
                message.sticky = true;
                message.error = false;
                message.content = "WAITING " + (timeToStart > 0 ? "- " + timeToStart : "");
            }
        }
    }
    else {
        message.error = false;
        message.sticky = true;
        message.content = "YOU ARE A SPECTATOR";
        if (counter == 0) {
            message.content += "\nWAITING FOR GAME START";
        }
    }
    ctx.webkitImageSmoothingEnabled = false;
    ctx.mozImageSmoothingEnabled = false;
    ctx.imageSmoothingEnabled = false;
    var remainingMillis = timePerTick * counter;
    remainingHundredthsPretty = Math.floor(remainingMillis/10) % 100;
    var remainingSecs = Math.floor(remainingMillis/1000);
    remainingSecsPretty = remainingSecs % 60;
    remainingMins = Math.floor(remainingSecs/60);
    if (totalTicksPassed == 0) {
        remainingMins = 0;
        remainingSecsPretty = 0;
        remainingHundredthsPretty = 0;
    }
    requestAnimationFrame(mainloop);
    ctx.fillStyle = "black";
    ctx.fillRect(-5, -5, 10 + window.innerWidth, 10 + window.innerHeight);
    inventoryHovered = -1;
    if (myCastle) {
        canPlaceWall = (Math.abs(mousePos.gameX - myCastle.x) < 400) && (Math.abs(mousePos.gameY - myCastle.y) < 400) || (inventoryPlace != undefined && inventoryStuff[inventoryPlace].word == 'F');
        mine.forEach(index => {
            object = objects[index];
            if (object && object.value == "F"){
                canPlaceWall = canPlaceWall || (Math.abs(mousePos.gameX - object.x) < 400) && (Math.abs(mousePos.gameY - object.y) < 400);
            }
        });
    }
    else {
        canPlaceWall = false;
    }
    canPlaceCastle = true;
    Object.values(objects).forEach(object => {
        if (object){
            if (mousePos.gameX > object.x - object.w/2 - 300 &&
                mousePos.gameX < object.x + object.w/2 + 300 &&
                mousePos.gameY > object.y - object.h/2 - 300 &&
                mousePos.gameY < object.y + object.h/2 + 300) {
                canPlaceCastle = false;
            }
        }
    });
    if (isRTF && !isStratChange && counter != 0 && myCastle != undefined) {
        var dX = window.innerWidth/2 - mousePos.realX;
        var dY = window.innerHeight/2 - mousePos.realY;
        var magnitude = Math.sqrt(dX * dX + dY * dY);
        var angle = Math.atan2(dY, dX);
        angle += myCastle.angle;
        mousePos.gameX = myCastle.x - Math.cos(angle) * magnitude;
        mousePos.gameY = myCastle.y - Math.sin(angle) * magnitude;
    }
    else {
        mousePos.gameX = mousePos.realX + cX;
        mousePos.gameY = mousePos.realY + cY;
    }
    if (mousePos.gameX > gameSize) {
        mousePos.gameX = gameSize;
    }
    else if (mousePos.gameX < 0) {
        mousePos.gameX = 0;
    }

    if (mousePos.gameY > gameSize) {
        mousePos.gameY = gameSize;
    }
    else if (mousePos.gameY < 0) {
        mousePos.gameY = 0;
    }
////////////////////////////////////////////////////////////
    if (isRTF && !isStratChange && counter != 0 && myCastle) {
        ctx.translate(window.innerWidth/2, window.innerHeight/2);
        ctx.rotate(-myCastle.angle);
        ctx.translate(-window.innerWidth / 2, -window.innerHeight / 2);
    }
    ctx.translate(-cX, -cY);
    var jitterX = Math.round(Math.random() * jitter);
    var jitterY = Math.round(Math.random() * jitter);
    jitter *= 0.95;
    ctx.translate(jitterX, jitterY);
////////////////////////////////////////////////////////////
    ctx.strokeStyle = palette.MIDGREEN;
    ctx.setLineDash([40, 10]);
    ctx.fillStyle = palette.DARKGREEN;
    ctx.lineWidth = 3;
    ctx.drawImage(preloads.background, 0, 0);//ctx.fillRect(-2.5, -2.5, gameSize + 5, gameSize + 5);
    ctx.strokeRect(0, 0, gameSize, gameSize);
    ctx.setLineDash([]);
    ctx.fillStyle = "black";
    ctx.fillRect(mousePos.gameX - 5, mousePos.gameY - 5, 10, 10);

    hovered = undefined;
    Object.keys(objects).forEach(id => {
        var object = objects[id];
        if (object.value == "F"){
            ctx.fillStyle = "rgba(0, 0, 0, 0.1)";
            ctx.fillRect(object.x - 10, object.y - 10, 20, 20);
        }
        else if (object.value == "b"){
            ctx.fillStyle = "green";
            ctx.beginPath();
            ctx.arc(object.x, object.y, 5, 0, Math.PI * 2);
            ctx.fill();
        }
        else if (object.value == "r"){
            /*ctx.fillStyle = "green";
            ctx.globalAlpha = object.radStrength;
            ctx.fillRect(object.x + object.box.x1, object.y + object.box.y1, object.box.x2 - object.box.x1, object.box.y2 - object.box.y1);
            ctx.globalAlpha = 1;*/
        }
        else if (object.value == "B"){
            ctx.fillStyle = "grey";
            ctx.strokeStyle = "#333333";
            ctx.lineWidth = 5;
            ctx.rotate(object.angle);
            ctx.fillRect(object.x - object.w/2, object.y - object.h/2, object.w, object.h);
            ctx.strokeRect(object.x - object.w / 2, object.y - object.h / 2, object.w, object.h);
            ctx.rotate(-object.angle);
        }
        else if (object.value == "C"){
            ctx.fillStyle = "pink";
            ctx.fillRect(object.x - 15, object.y - 15, 30, 10);
            ctx.fillStyle = "gold";
            ctx.fillRect(object.x - 15, object.y - 5, 30, 20);
        }
        else if (object.value == "0") {
            ctx.strokeStyle = "red";
            ctx.fillStyle = "brown";
            ctx.lineWidth = 3;
            ctx.beginPath();
            ctx.translate(object.x, object.y);
            ctx.rotate(object.angle);
            ctx.moveTo(0, 0);
            ctx.arc(0, 0, 5, 0, Math.PI * 2);
            ctx.lineTo(25, 0);
            ctx.rotate(-object.angle);
            ctx.translate(-object.x, -object.y);
            ctx.stroke();
            ctx.fill();
        }
        else if (object.value == "1") {
            ctx.strokeStyle = "white";
            ctx.fillStyle = "brown";
            ctx.lineWidth = 3;
            ctx.beginPath();
            ctx.translate(object.x, object.y);
            ctx.rotate(object.angle);
            ctx.moveTo(0, 0);
            ctx.arc(0, 0, 5, 0, Math.PI * 2);
            ctx.lineTo(25, 0);
            ctx.rotate(-object.angle);
            ctx.translate(-object.x, -object.y);
            ctx.stroke();
            ctx.fill();
        }
        else if (object.value == "2") {
            ctx.strokeStyle = "black";
            ctx.fillStyle = "white";
            ctx.lineWidth = 3;
            ctx.beginPath();
            ctx.translate(object.x, object.y);
            ctx.rotate(object.angle);
            ctx.moveTo(0, 0);
            ctx.arc(0, 0, 5, 0, Math.PI * 2);
            ctx.lineTo(25, 0);
            ctx.rotate(-object.angle);
            ctx.translate(-object.x, -object.y);
            ctx.stroke();
            ctx.fill();
        }
        else if (object.value == "3") {
            ctx.strokeStyle = "magenta";
            ctx.fillStyle = "yellow";
            ctx.lineWidth = 3;
            ctx.beginPath();
            ctx.translate(object.x, object.y);
            ctx.arc(0, 0, 20, 0, Math.PI * 2);
            ctx.translate(-object.x, -object.y);
            ctx.fill();
        }
        else {
            art.drawObj(object);
        }
        var teamy = object != myCastle && myCastle && teamLeader && teams[myCastle.banner] && teams[myCastle.banner] == teams[object.banner] && object.value == "R";
        if (mousePos.gameX > object.x - 10 && mousePos.gameX < object.x + 10 && mousePos.gameY > object.y - 10 && mousePos.gameY < object.y + 10) {
            if ((mine.indexOf(object.id) != -1) || teamy) {
                if (object.isEditable || teamy){
                    object.hovered = true;
                    ctx.strokeStyle = "yellow";
                    ctx.lineWidth = 3;
                    ctx.strokeRect(object.x - 10, object.y - 10, 20, 20);
                    hovered = object;
                }
            }
            else {
                ctx.fillStyle = palette.RED;
                ctx.font = "10px sans-serif";
                var team = "Team " + banners[teams[object.banner]];
                var size = ctx.measureText(banners[object.banner]).width;
                if (team){
                    size = Math.max(size, ctx.measureText(team).width);
                }
                ctx.fillRect(mousePos.gameX - size - 5, mousePos.gameY - 15, size + 10, 20 * (team ? 2 : 1));
                ctx.textAlign = "left";
                ctx.fillStyle = "black";
                ctx.fillText(banners[object.banner], mousePos.gameX - size, mousePos.gameY);
                if (team){
                    ctx.fillText(team, mousePos.gameX - size, mousePos.gameY + 20);
                }
            }
        }
        if (teamy || (object.isEditable && mine.indexOf(object.id) != -1)){
            ctx.lineWidth = 2;
            ctx.setLineDash([6, 2]);
            ctx.strokeStyle = palette.MIDGREEN;
            ctx.beginPath();
            ctx.moveTo(object.x, object.y);
            ctx.lineTo(object.goalPos.x, object.goalPos.y);
            ctx.stroke();
            ctx.setLineDash([]);
            ctx.fillStyle = palette.LIGHTBLUE;
            ctx.beginPath();
            ctx.arc(object.goalPos.x, object.goalPos.y, 3, 0, Math.PI * 2);
            ctx.fill();
            ctx.lineWidth = 3;
            ctx.strokeStyle = palette.LIGHTBLUE;
            ctx.beginPath();
            ctx.save();
            ctx.translate(object.goalPos.x, object.goalPos.y);
            ctx.rotate(object.goalPos.angle);
            ctx.moveTo(5, 0);
            ctx.lineTo(5 + 11, 0);
            ctx.restore();
            ctx.stroke();
        }
    });
    if (exploder.time > 0){
        exploder.time --;
        ctx.fillStyle = "red";
        ctx.fillRect(exploder.x - 100, exploder.y - 100, 200, 200);
    }
    if (isStratChange){
        if (selected) {
            if (angleAdjustMode) {
                selected.goalPos.angle = Math.atan2(mousePos.gameY - selected.goalPos.y, mousePos.gameX - selected.goalPos.x);
            }
            else {
                selected.goalPos.x = mousePos.gameX;
                selected.goalPos.y = mousePos.gameY;
            }
            sendProtocolMessage("m", [selected.id, selected.goalPos.x, selected.goalPos.y, selected.goalPos.angle]);
        }
    }
    else{
        selected = undefined;
        angleAdjustMode = false;
        hovered = undefined;
        inventoryPlace = undefined;
    }
/////////////////////////////////////////////////////////
    ctx.translate(cX, cY);
    ctx.translate(-jitterX, -jitterY);
    if (isRTF && !isStratChange && counter != 0 && myCastle != undefined) {
        ctx.translate(window.innerWidth / 2, window.innerHeight / 2);
        ctx.rotate(myCastle.angle);
        ctx.translate(-window.innerWidth / 2, -window.innerHeight / 2);
    }
/////////////////////////////////////////////////////////
    if (keysDown["ArrowUp"] || keysDown["w"]){
        yv -= 5;
    }
    if (keysDown["ArrowDown"] || keysDown["s"]){
        yv += 5;
    }
    if (keysDown["ArrowLeft"] || keysDown["a"]){
        xv -= 5;
    }
    if (keysDown["ArrowRight"] || keysDown["d"]){
        xv += 5;
    }
    yv *= 0.8;
    xv *= 0.8;
    cY += yv;
    cX += xv;
    if (cX < -window.innerWidth/2){
        cX = -window.innerWidth/2;
    }
    if (cX > gameSize - window.innerWidth/2){
        cX = gameSize - window.innerWidth/2;
    }
    if (cY < -window.innerHeight/2){
        cY = -window.innerHeight/2;
    }
    if (cY > gameSize - window.innerHeight/2){
        cY = gameSize - window.innerHeight/2;
    }
    drawSidebar();
    drawMessage();
    drawUpgradeMenu();
    if (myCastle && myCastle.health) {
        /*ctx.fillStyle = "white";
        ctx.globalAlpha = 1;
        ctx.font = "bold 40px sans-serif";
        ctx.textAlign = "left";
        ctx.fillText(myCastle.health, 40, 40);*/
        ctx.fillStyle = "blue";
        ctx.fillRect(window.innerWidth/2 - 100, 50, 200, 20);
        ctx.fillStyle = "red";
        ctx.fillRect(window.innerWidth/2 - 98, 52, myCastle.health / 3 * 196, 16);
    }
    if (effect == "CRT"){
        ctx.beginPath();
        ctx.strokeStyle = "black";
        ctx.lineWidth = 0.5;
        const lineCount = 50;
        for (var y = 0; y < lineCount; y ++){
            var linePos = y * window.innerHeight/lineCount;
            linePos += CRTEffectWipe % (window.innerHeight/lineCount);
            CRTEffectWipe += 0.01;
            ctx.moveTo(0, linePos);
            ctx.lineTo(window.innerWidth, linePos);
        }
        ctx.stroke();
        drawCRT();
    }
}

function play() {
    document.getElementById("mainscreen").style.display = "none";
    document.getElementById("game").style.display = "";

    window.addEventListener("mouseup", (evt) => {
        if (didSelectHome){
            if (inventoryHovered != -1) {
                if (inventoryStuff[inventoryHovered].cbk){
                    inventoryStuff[inventoryHovered].cbk();
                    inventoryStuff[inventoryHovered].stack --;
                }
                else {
                    if (inventoryStuff[inventoryHovered].place) {
                        inventoryPlace = inventoryHovered;
                        inventoryHovered = -1;
                    }
                    else {
                        sendProtocolMessage("p", [inventoryStuff[inventoryHovered].word, (myCastle.x + (Math.random() * 300 - 150)), (myCastle.y + (Math.random() * 300 - 150))]);
                    }
                }
                inventoryMode = false;
            }
            else if (inventoryPlace != undefined) {
                if (!cantPlace) {
                    sendProtocolMessage("p", [inventoryStuff[inventoryPlace].word, Math.round(mousePos.gameX), Math.round(mousePos.gameY)]);
                    inventoryPlace = undefined;
                }
            }
            else if (isStratChange){
                if (angleAdjustMode) {
                    angleAdjustMode = false;
                    selected = undefined;
                    hovered = undefined;
                }
                else {
                    if (selected) {
                        angleAdjustMode = true;
                    }
                    else if (hovered) {
                        hovered.selected = true;
                        selected = hovered;
                    }
                }
            }
        }
        else {
            if (canPlaceCastle){
                sendProtocolMessage("p", ["c", Math.round(mousePos.gameX), Math.round(mousePos.gameY)]); // place castle at position
                didSelectHome = true;
            }
        }
    });                
    mainloop();
}

window.onresize = () => {
    var canvas = document.getElementById("game");
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    if (effect == "CRT") {
        canvas = document.getElementById("old_game");
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
    }
};

window.onresize();