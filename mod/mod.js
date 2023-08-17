const DEBUG = false;
const INTERPOLATE = true;

function clamp(min, val, max) {
    if (val < min) {
        val = min;
    }
    if (val > max) {
        val = max;
    }
    return val;
}

function mulberry32(a) {
    return function () {
        var t = a += 0x6D2B79F5;
        t = Math.imul(t ^ t >>> 15, t | 1);
        t ^= t + Math.imul(t ^ t >>> 7, t | 61);
        return ((t ^ t >>> 14) >>> 0) / 4294967296;
    }
}

function randomizeBanner() {
    var banner = document.getElementById("banner");
    if (banner.value == "") {
        function rando(list) {
            return list[Math.floor(Math.random() * list.length)];
        }
        var spacer = rando([" ", "_", "-", ".", "+"]);
        var adjectives = ["Proud", "Angry", "Small", "Floral", "Wet", "Green", "Brown", "Black", "Strong", "Weak", "Limping", "Hungry"];
        var animals = ["Bear", "Wolf", "Rabbit", "Deer", "Squirrel", "Eagle", "Sparrow", "Mouse", "Hawk"];
        var junctions = ["Makes", "of", "Walks" + spacer + "in", "Eats" + spacer + "the", "Runs" + spacer + "in", "Flies" + spacer + "in", "Binge" + spacer + "Watches"];
        var elements = ["Wind", "Water", "Fire", "Dirt", "Clay", "Trees", "Costco"];
        var name = rando(adjectives) + spacer + rando(animals) + spacer + rando(junctions) + spacer + rando(elements) + spacer + Math.round(Math.random() * 10000);
        banner.value = name;
    }
}

function todo(thing) {
    throw "TODO: " + thing;
}


function notify(notification) {
    if (!document.hasFocus()) {
        const notif = new Notification("MMOSG", {
            body: notification
        });
    }
}


class ProtocolMessageReceivedEvent extends Event {
    constructor(command, args, initDict = undefined) {
        super("protocolmessagereceived", initDict);
        this.command = command;
        this.args = args;
    }
}


class Protocol extends EventTarget {
    constructor(socket, onmessage) {
        super();
        this.socket = socket;
        this.synchCallbach = onmessage;
        socket.onmessage = (message) => { this.message_recvd(message) };
        setInterval(() => {
            this.isOnline = this.pong; // if it's received a pong in 500ms, it's online; otherwise, it is not. Either way send a ping.
            this.ping();
            this.pong = false; // After sending, we can't have received a pong!
        }, 500);
        this.isOnline = false;
        this.rtfHasEmptied = true;
    }

    message_recvd(raw) {
        this.pong = true; // If it's receiving anything, it's online
        if (raw.data) {
            var data = raw.data;
            if (data == "_") { // If it's a pong message, do nothing, this is not meant for the game handler to see
                return;
            }
            var command = data[0];
            var args = [];
            var i = 1;
            var buffer = "";
            while (i < data.length) {
                var iEnd = i + data.charCodeAt(i);
                while (i < iEnd) {
                    i++;
                    buffer += data[i];
                }
                args.push(buffer);
                buffer = "";
                i++;
            }
            if (command == "d") {
                console.log("DELETED" + args[0]);
            }
            this.dispatchEvent(new ProtocolMessageReceivedEvent(command, args));
            if (this.synchCallbach) {
                this.synchCallbach(command, args);
            }
        }
    }

    send(command, args) {
        if (!Array.isArray(args)) {
            args = [args]; // Make sure args is a list
        }
        for (var i = 0; i < args.length; i++){
            args[i] = "" + args[i]; // Convert all the arguments to strings
        }
        var message = command;
        args.forEach(arg => {
            message += String.fromCharCode(arg.length);
            message += arg;
        });
        this.socket.send(message);
    }

    place(type, x, y) {
        this.send('p', [type, x, y]);
    }

    ping() {
        this.socket.send("_");
    }

    connect(banner, password, playMode, isSpectating) {
        this.send('c', [
            isSpectating ? "" : password,
            banner,
            isSpectating ? "" : playMode
        ]);
    }

    move(id, x, y, a) {
         this.send("m", [id, x, y, a]);
    }

    cost(amount) {
        this.send("C", [amount]);
    }

    rtf(thrust, left, right, brake, shoot) {
        if (thrust || left || right || brake || shoot) { // don't ever send an empty R frame, that's stupid
            thrust = thrust ? "1" : "0";
            left   = left   ? "1" : "0";
            right  = right  ? "1" : "0";
            brake  = brake  ? "1" : "0";
            shoot  = shoot  ? "1" : "0";
            this.send("R", [thrust, left, right, brake, shoot]);
            this.rtfHasEmptied = false;
        }
        else if (!this.rtfHasEmptied) {
            this.send("R", ["0", "0", "0", "0", "0"]);
            this.rtfHasEmptied = true;
        }
    }

    talk(message) { // This function will see significant changes as I actually implement chatroom!
        this.send("T", [message]);
    }

    upgrade(id, upgrade) {
        this.send("U", [id, upgrade]);
    }
}


class Sidebar {
    constructor() {
        this.path = new Path2D();
        this.path.moveTo(60, 56);
        this.path.quadraticCurveTo(36, 56, 36, 80);
        this.path.lineTo(36, 262);
        this.path.quadraticCurveTo(36, 286, 60, 286);
        this.path.lineTo(242, 286);
        this.path.quadraticCurveTo(266, 286, 266, 310);
        this.path.lineTo(266, 856);
        this.path.quadraticCurveTo(266, 872, 250, 880);
        this.path.lineTo(152, 928);
        this.path.quadraticCurveTo(136, 936, 136, 952);
        this.path.lineTo(136, 1120);
        this.path.quadraticCurveTo(136, 1144, 160, 1144);
        this.path.lineTo(0, 1144);
        this.path.lineTo(0, 56);
        this.path.closePath();
        this.dumpass = new Path2D(); // Whatever you think the name is, you are wrong, it isn't
        this.dumpass.moveTo(266, 910);
        this.dumpass.quadraticCurveTo(266, 897, 254, 903);
        this.dumpass.lineTo(158, 950);
        this.dumpass.quadraticCurveTo(152, 953, 152, 958);
        this.dumpass.lineTo(152, 1120);
        this.dumpass.quadraticCurveTo(152, 1129, 170, 1129);
        this.dumpass.lineTo(258, 1129);
        this.dumpass.quadraticCurveTo(266, 1129, 266, 1121);
        this.dumpass.lineTo(266, 910);
        this.isInventory = false;
        this.scrollHeight = 1144;
        this.tick = 0;
    }

    drawInventory(parent) {
        const curveRadius = 24;
        const inventoryObjectHeight = 75;
        var ctx = parent.ctx;
        var inventoryTotalHeight = Math.max(parent.inventory.length * inventoryObjectHeight + 57, window.innerHeight - 56);
        this.scrollHeight = inventoryTotalHeight;
        ctx.beginPath();
        ctx.fillStyle = "black";
        ctx.moveTo(0, 56);
        ctx.lineTo(266 + curveRadius, 56);
        ctx.quadraticCurveTo(266, 56, 266, 56 + curveRadius);
        ctx.lineTo(266, inventoryTotalHeight - curveRadius);
        ctx.quadraticCurveTo(266, inventoryTotalHeight, 266 + curveRadius, inventoryTotalHeight);
        ctx.lineTo(0, inventoryTotalHeight);
        ctx.closePath();
        ctx.fill();
        parent.inventory.forEach((item, i) => {
            var rootY = 57 + i * inventoryObjectHeight;
            if (item.selected) {
                ctx.fillStyle = "#440000";
                ctx.fillRect(0, rootY, 266, inventoryObjectHeight);
            }
            ctx.font = "14px 'Chakra Petch'";
            ctx.fillStyle = "white";
            ctx.textAlign = "left";
            ctx.fillText(item.name, 30, rootY + 14);
            ctx.fillStyle = "white";
            var width = ctx.measureText(item.cost + "").width;
            ctx.fillRect(246 - width, rootY, width, 18);
            ctx.fillStyle = "black";
            ctx.fillText(item.cost, 246 - width, rootY + 14);
            ctx.fillStyle = "blue";
            ctx.font = "10px 'Chakra Petch'";
            ctx.fillText(item.descriptionL1, 20, rootY + 40);
            ctx.fillText(item.descriptionL2, 20, rootY + 50);
            item.hovered = false;
            if (parent.status.score >= item.cost && item.stack != 0) {
                if (parent.mouseX < 266 && parent.mouseY + parent.sideScroll > rootY && parent.mouseY + parent.sideScroll < rootY + inventoryObjectHeight) {
                    ctx.lineWidth = 1;
                    ctx.strokeStyle = "white";
                    ctx.strokeRect(0, rootY, 266, inventoryObjectHeight);
                    item.hovered = true;
                }
            }
        });
    }

    draw(parent, interpolator) {
        var ctx = parent.ctx;
        if (this.isInventory) {
            this.drawInventory(parent);
            return;
        }
        ctx.fillStyle = "black";
        ctx.fill(this.path);
        ctx.beginPath();
        ctx.roundRect(46, 66, 220, 210, 14);
        ctx.fill();
        ctx.fill(this.dumpass);
        ctx.lineWidth = 1;
        ctx.strokeStyle = "white";
        ctx.stroke(this.dumpass);
        ctx.font = "bold 14px 'Chakra Petch'";
        ctx.textAlign = "left";
        ctx.fillStyle = "white";
        ctx.fillText("X", 20, 632 + 7);
        ctx.fillText("Y", 148, 632 + 7);
        ctx.font = "24px 'Chakra Petch'";
        ctx.fillText(parent.gameX, 20, 650 + 12);
        ctx.fillText(parent.gameY, 148, 650 + 12);
        ctx.fillStyle = "#333";
        ctx.fillRect(18, 487, 218, 2);
        ctx.fillRect(18, 771, 218, 2);

        ctx.strokeStyle = "white";
        ctx.fillStyle = "white";
        ctx.lineWidth = 1;
        ctx.strokeRect(197, 1060, 26, 26);
        if (parent.controls.up) {
            ctx.fillRect(197, 1060, 26, 26);
        }    
        ctx.strokeRect(168, 1089, 26, 26);
        if (parent.controls.left) {
            ctx.fillRect(168, 1089, 26, 26);
        }
        ctx.strokeRect(197, 1089, 26, 26);
        if (parent.controls.down) {
            ctx.fillRect(197, 1089, 26, 26);
        }
        ctx.strokeRect(226, 1089, 26, 26);
        if (parent.controls.right) {
            ctx.fillRect(226, 1089, 26, 26);
        }
        if (parent.castle) {
            ctx.strokeRect(226, 1060, 26, 26);
            ctx.fillRect(226, 1060, 26, 26);
            ctx.beginPath();
            ctx.save();
            ctx.translate(238, 1072);
            var ang = Math.atan2(parent.castle.y - parent.gameY, parent.castle.x - parent.gameX);
            ctx.rotate(ang);
            ctx.moveTo(6, 0);
            ctx.lineTo(-6, -3);
            ctx.lineTo(-4, 0);
            ctx.lineTo(-6, 3);
            ctx.closePath();
            ctx.fillStyle = "black";
            ctx.fill();
            ctx.restore();
            ctx.beginPath();
            ctx.arc(209, 992, 50, 0, Math.PI * 2);
            ctx.lineWidth = 1;
            ctx.strokeStyle = "white";
            ctx.stroke();
            ctx.beginPath();
            ctx.arc(209, 992, 4, 0, Math.PI * 2);
            ctx.fillStyle = "white";
            ctx.fill();
            ctx.globalAlpha = 0.2;
            ctx.beginPath();
            ctx.arc(209, 992, 33, 0, Math.PI * 2);
            ctx.moveTo(226, 992);
            ctx.arc(209, 992, 17, 0, Math.PI * 2);
            ctx.stroke();
            var nearestValue = Infinity;
            Object.values(parent.objects).forEach(object => {
                var dx = object.getX(interpolator) - parent.castle.getX(interpolator);
                var dy = object.getY(interpolator) - parent.castle.getY(interpolator);
                var dist = dx * dx + dy * dy;
                if (!object.isOurs && dist < nearestValue && object.type != 'b' && object.isCompassVisible()) {
                    nearestValue = dist;
                }
                if (dist < 400 * 400 && object.isCompassVisible() && object != parent.castle) {
                    if (object.isOurs) {
                        ctx.fillStyle = "rgb(47, 237, 51)";
                    }
                    else {
                        ctx.fillStyle = "rgb(231, 57, 30)";
                    }
                    var offsetX = (dx / 400) * 50;
                    var offsetY = (dy / 400) * 50;
                    ctx.beginPath();
                    ctx.arc(209 + offsetX, 992 + offsetY, 5, 0, Math.PI * 2);
                    ctx.globalAlpha = 0.5;
                    ctx.fill();
                    ctx.globalAlpha = 1;
                    ctx.beginPath();
                    ctx.arc(209 + offsetX, 992 + offsetY, 2, 0, Math.PI * 2);
                    ctx.fill();
                }
            });
            ctx.globalAlpha = 1;
        }
        ctx.beginPath();
        ctx.moveTo(209, 1069);
        ctx.lineTo(214, 1075);
        ctx.lineTo(204, 1075);
        ctx.closePath()
        ctx.moveTo(183, 1096);
        ctx.lineTo(177, 1101);
        ctx.lineTo(183, 1106);
        ctx.closePath();
        ctx.moveTo(204, 1098);
        ctx.lineTo(214, 1098);
        ctx.lineTo(209, 1104);
        ctx.closePath();
        ctx.moveTo(235, 1096);
        ctx.lineTo(241, 1101);
        ctx.lineTo(235, 1106);
        ctx.closePath();
        ctx.fillStyle = "black";
        ctx.fill();
        ctx.stroke();

        this.drawSquaresReadout(ctx, parent.castle ? 1 - (parent.health / 3) : 1, 18, 945);
        this.drawSquaresReadout(ctx, 0.5, 54, 945);
        this.drawSquaresReadout(ctx, 1 - clamp(0, nearestValue / (800 * 800), 1), 90, 945)
        for (var i = 0; i < 33; i++) {
            if (i < 9) {
                ctx.fillStyle = "red";
            }
            else if (i < 19) {
                ctx.fillStyle = "#F3BB38";
            }
            else if (i < 29) {
                ctx.fillStyle = "#2FED33";
            }
            else {
                ctx.fillStyle = "white";
            }
            ctx.fillRect(42, 945 + i * 6, 4, 2);
            ctx.fillRect(78, 945 + i * 6, 4, 2);
        }
        ctx.fillStyle = "red";
        ctx.fillRect(18, 999, 88, 2);
        this.scrollHeight = 1144;

        ctx.fillStyle = "white";
        ctx.font = "12px 'Chakra Petch'";
        ctx.fillText("EARLY WARNING SYSTEM", 42, 852);
        var lert = clamp(0, nearestValue / (1200 * 1200), 1);
        var lengths = [
            218,
            203,
            183,
            155,
            120,
            78
        ];
        lengths.forEach(item => {
            this.drawBuuchie(ctx, "rgb(" + 255 * ((1 - lert) + (1 - (item - 78)/140) * 0.5) + ",0,0)", item - 18);
        });
        ctx.beginPath();
        ctx.strokeStyle = "white";
        ctx.lineWidth = 1;
        ctx.moveTo(18, 884);
        ctx.lineTo(185, 884);
        ctx.moveTo(18, 905);
        ctx.lineTo(153, 905);
        ctx.stroke();
        ctx.fillStyle = "black";
        ctx.fillRect(17, 884, 185, 20);
        var error = "OK";
        if (nearestValue < 600 * 600) { // it's nearby
            error = "NEAR THRESHOLD";
        }
        if (nearestValue < 400 * 400) {
            error = "CLOSE PROXIMITY";
            this.tick++; // make the flash twice as fast if you're in close proximity.
        }
        this.tick++;
        if (this.tick > 60) {
            this.tick = 0;
        }
        ctx.font = "12px 'Chakra Petch'";
        ctx.fillStyle = "white";
        ctx.fillText(error, 61, 899);
        if (error != "OK") {
            if (this.tick > 30) {
                ctx.fillStyle = "red";
                ctx.fillRect(18, 887, 39, 16);
                ctx.fillStyle = "white";
                ctx.font = "bold 12px 'Chakra Petch'";
                ctx.fillText("WARN", 20, 899);
            }
        }
        this.availability(ctx, "CASTLE PLACEMENT", 779, !parent.status.mouseWithinNarrowField);
        this.availability(ctx, "OBJECT PLACEMENT", 803, parent.status.canPlaceObject);

        Object.values(parent.objects).forEach(obj => {
            var scaleX = 220 / parent.gamesize;
            var scaleY = 210 / parent.gamesize
            var convertedX = 46 + obj.x * scaleX;
            var convertedY = 66 + obj.y * scaleY;
            var convertedW = obj.w * scaleX;
            var convertedH = obj.h * scaleY;
            if (!obj.isCompassVisible()) {
                return;
            }
            if (obj.isOurs) {
                ctx.fillStyle = "rgb(47, 237, 51)";
            }
            else {
                ctx.fillStyle = "rgb(231, 57, 30)";
            }
            if (obj.type == "R" || obj.type == "c") {
                ctx.beginPath();
                ctx.moveTo(convertedX, convertedY - 3);
                ctx.lineTo(convertedX + 3, convertedY);
                ctx.lineTo(convertedX, convertedY + 3);
                ctx.lineTo(convertedX - 3, convertedY);
                ctx.closePath();
                ctx.fill();
            }
            else if (obj.type == "B") {
                ctx.fillStyle = "#555";
                ctx.fillRect(convertedX - convertedW/2, convertedY - convertedH/2, convertedW, convertedH);
            }
            else {
                ctx.beginPath();
                ctx.arc(convertedX, convertedY, 3, 0, Math.PI * 2);
                ctx.globalAlpha = 0.5;
                ctx.fill();
                ctx.globalAlpha = 1;
                ctx.beginPath();
                ctx.arc(convertedX, convertedY, 1, 0, Math.PI * 2);
                ctx.fill();
            }
            ctx.fillStyle = "white";
            ctx.fillRect(46 + parent.gameX * scaleX - 1, 66 + parent.gameY * scaleY - 1, 2, 2);
        });
    }

    availability(ctx, title, rootY, value) {
        ctx.font = "bold 12px 'Chakra Petch'";
        ctx.fillStyle = "white";
        ctx.fillText(title, 20, rootY + 12);
        ctx.fillStyle = value ? "#2FED33" : "#E7391E";
        ctx.fillRect(153, rootY, value ? 66 : 82, 16);
        ctx.fillStyle = value ? "black" : "white";
        ctx.fillText(value ? "AVAILABLE" : "UNAVAILABLE", 155, rootY + 12);
    }

    drawBuuchie(ctx, color, pos) {
        ctx.strokeStyle = "white";
        ctx.fillStyle = color;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(18, 869);
        ctx.quadraticCurveTo(18, 861, 26, 861);
        ctx.lineTo(18 + pos - 8, 861);
        ctx.quadraticCurveTo(18 + pos, 861, 18 + pos - (8/68 * pos/2), 869);
        ctx.lineTo(18 + pos / 2, 861 + 68);
        ctx.lineTo(18, 861 + 68)
        ctx.closePath();
        ctx.fill();
        ctx.stroke();
    }

    drawSquaresReadout(ctx, valueOf, rootX, rootY) {
        valueOf = clamp(0, valueOf, 1);
        valueOf = 1 - valueOf;
        valueOf *= 33;
        for (var i = 0; i < 33; i++) {
            if (i >= valueOf) {
                if (i < 9) {
                    ctx.fillStyle = "red";
                }
                else {
                    ctx.fillStyle = "#B8B8B8";
                }
            }
            else {
                ctx.fillStyle = "#222";
            }
            ctx.fillRect(rootX, rootY + i * 6, 16, 2);
        }
    }

    clickies(parent) {
        this.inventorySelected = undefined;
        parent.inventory.forEach(item => {
            if (item.hovered && (parent.status.moveShips || parent.status.isRTF)) {
                if (item.selected) {
                    item.selected = false;
                }
                else {
                    if (item.place.word) { // it's an object to place
                        this.inventorySelected = item;
                        item.selected = true;
                    }
                    else {
                        if (item.stack) {
                            item.stack--;
                        }
                        parent.comms.cost(item.cost);
                    }
                    if (item.place.cbk) {
                        item.place.cbk();
                    }
                    if (item.place.upgrade) {
                        if (item.place.upgrade.effect == "castle") {
                            parent.comms.upgrade(parent.castle.id, item.place.upgrade.word);
                        }
                    }
                }
            }
            else {
                item.selected = false;
            }
        });
    }
}


class GameObject {
    constructor(x, y, w, h, a, type, editable, id, banner) {
        this.x = x;
        this.xOld = x;
        this.y = y;
        this.yOld = y;
        this.w = w;
        this.wOld = w;
        this.h = h;
        this.hOld = h;
        this.a = a;
        this.aOld = a;
        this.type = type;
        this.isEditable = editable;
        this.id = id;
        this.banner = banner;
        this.upgrades = [];
        this.goalPos = {
            x: this.x,
            y: this.y,
            a: this.a,
            pathLen: 0,
            initialX: this.x,
            initialY: this.y,
            hasChanged: false
        };
        this.box = this.bbox();
        this.isOurs = false;
        this.isHovered = false;
        this.bodyHovered = false;
        this.editState = 0; // 0 = none; 1 = picked up, moving; 2 = picked up, setting angle
        this.didMove = true; // whether or not it's moved since last tick
    }

    getTypeString() {
        if (this.type == "c") {
            return "CASTLE";
        }
        if (this.type == "R") {
            return "RTF";
        }
        if (this.type == "w") {
            return "WALL";
        }
        if (this.type == "b") {
            return "BULLET";
        }
        if (this.type == "C") {
            return "CHEST";
        }
        return "SHIP";
    }

    getFriendlinessString() {
        if (this.isOurs) {
            return "FRIENDLY";
        }
        else if (this.banner == 0) {
            return "NEUTRAL";
        }
        return "ENEMY";
    }

    interpolate(value, property) {
        return this[property] * value + this[property + "Old"] * (1 - value);
    }

    getX(interpolator) {
        return this.interpolate(interpolator, "x");
    }

    getY(interpolator) {
        return this.interpolate(interpolator, "y");
    }

    getA(interpolator) {
        return this.interpolate(interpolator, "a");
    }

    getW(interpolator) {
        return this.interpolate(interpolator, "w");
    }

    getH(interpolator) {
        return this.interpolate(interpolator, "h");
    }

    draw(master, interpolator, zoomLevel = 1) {
        var ctx = master.ctx;
        ctx.lineWidth = 2;
        ctx.globalAlpha = 1;
        ctx.strokeStyle = "white";
        var w = this.getW(interpolator) * zoomLevel;
        var h = this.getH(interpolator) * zoomLevel;
        var a = this.getA(interpolator);
        var x = this.getX(interpolator) * zoomLevel;
        var y = this.getY(interpolator) * zoomLevel;
        ctx.translate(x, y);
        ctx.rotate(a);
        if (this.type == "R") {
            ctx.fillStyle = "white";
            ctx.fillRect(-w / 2, -h / 2, w, 5);
            ctx.strokeRect(-w / 2, -h / 2, w, h);
        }
        else if (this.type == "c") {
            ctx.drawImage(document.querySelector("img#castle"), -35, -35);
        }
        else if (this.type == "C") {
            ctx.beginPath();
            ctx.drawImage(document.querySelector("img#chest"), -11, -12);
        }
        else if (this.type == "f") {
            ctx.rotate(Math.PI / 2);
            ctx.drawImage(document.querySelector("img#ship"), -15, -15);
            ctx.rotate(-Math.PI / 2);
        }
        else if (this.type == "b") {
            ctx.fillStyle = "white";
            ctx.fillRect(-w / 2, -h / 2, w, h);
        }
        else if (this.type == "w") {
            ctx.drawImage(document.querySelector("img#wall"), -10, -10);
        }
        else {
            ctx.strokeRect(-w / 2, -h / 2, w, h);
        }
        ctx.rotate(-a);
        ctx.font = "10px 'Chakra Petch'";
        if (this.isOurs) {
            ctx.fillStyle = "orange";
            ctx.font = "bold 12px 'Chakra Petch'";
        }
        else{
            ctx.fillStyle = "yellow";
        }
        ctx.textAlign = "left";
        if (DEBUG) {
            ctx.fillText(this.type + "#" + this.id + " " + this.banner + "(" + master.banners[this.banner] + ")", -50, -h / 2 - 10);
        }
        ctx.translate(-x, -y);
        if (DEBUG) {
            ctx.strokeStyle = "green";
            ctx.beginPath();
            ctx.moveTo(x, y);
            ctx.lineTo(this.x * zoomLevel, this.y * zoomLevel);
            ctx.stroke();
        }
        if (DEBUG) {
            ctx.globalAlpha = 1;
            ctx.strokeStyle = "blue";
            ctx.lineWidth = 1;
            ctx.strokeRect(this.box[0] * zoomLevel, this.box[1] * zoomLevel, this.box[2] * zoomLevel - this.box[0] * zoomLevel, this.box[3] * zoomLevel - this.box[1] * zoomLevel);
        }
        if (this.isOurs && this.isEditable) {
            ctx.strokeStyle = "green";
            ctx.fillStyle = "green";
            if (this.isHovered) {
                ctx.fillRect(this.goalPos.x * zoomLevel - 5, this.goalPos.y * zoomLevel - 5, 10, 10);
            }
            ctx.strokeRect(this.goalPos.x * zoomLevel - 5, this.goalPos.y * zoomLevel - 5, 10, 10);
            ctx.beginPath();
            ctx.moveTo(x, y);
            ctx.setLineDash([2, 4]);
            ctx.strokeStyle = "white";
            ctx.fillStyle = "#F3BB38";
            ctx.lineWidth = 1;
            ctx.lineTo(this.goalPos.x * zoomLevel, this.goalPos.y * zoomLevel);
            ctx.stroke();
            ctx.beginPath();
            ctx.setLineDash([]);
            ctx.translate(this.goalPos.x * zoomLevel, this.goalPos.y * zoomLevel);
            ctx.rotate(this.goalPos.a + Math.PI/2);
            var gradient = ctx.createLinearGradient(14, -14, 14, 14);
            gradient.addColorStop(0.0, "#F3BB38");
            gradient.addColorStop(0.5, "transparent");
            ctx.strokeStyle = gradient;
            //ctx.moveTo(this.goalPos.x * zoomLevel, this.goalPos.y * zoomLevel);
            //ctx.lineTo((this.goalPos.x + Math.cos(this.goalPos.a) * 20) * zoomLevel, (this.goalPos.y + Math.sin(this.goalPos.a) * 20) * zoomLevel);
            ctx.arc(0, 0, 14, 0, Math.PI * 2);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(0, -18);
            ctx.lineTo(2, -13);
            ctx.lineTo(-2, -13);
            ctx.closePath();
            ctx.fill();
            ctx.rotate(-this.goalPos.a - Math.PI/2);
            ctx.translate(-this.goalPos.x * zoomLevel, -this.goalPos.y * zoomLevel);
        }
        if (this.bodyHovered) {
            ctx.fillStyle = "#F3BB38";
            ctx.font = "8px 'Chakra Petch'";
            var tooltipHeight = this.isOurs ? 40 : 30;
            var tooltipWidth = Math.max(41, ctx.measureText(master.banners[this.banner]).width + 4);
            ctx.fillRect(x - 40 - tooltipWidth, y - tooltipHeight/2, tooltipWidth, tooltipHeight);
            ctx.beginPath();
            ctx.moveTo(x - 41, y - 5);
            ctx.lineTo(x - 35, y);
            ctx.lineTo(x - 41, y + 5);
            ctx.closePath();
            ctx.fill();
            ctx.fillStyle = "black";
            ctx.fillText(this.getTypeString(), x - 38 - tooltipWidth, y - tooltipHeight/2 + 9);
            ctx.fillText(this.getFriendlinessString(), x - 38 - tooltipWidth, y - tooltipHeight / 2 + 18);
            ctx.fillText(master.banners[this.banner], x - 38 - tooltipWidth, y - tooltipHeight / 2 + 27)
            if (this.isOurs) {
                var dx = this.x - this.goalPos.initialX;
                var dy = this.y - this.goalPos.initialY;
                dx *= dx;
                dy *= dy;
                var displacement = Math.sqrt(dx + dy);
                var toPrint = Math.round((displacement / this.goalPos.pathLen) * 100) + "%";
                if (this.goalPos.pathLen == 0) {
                    toPrint = "0%";
                    if (this.goalPos.displacement == 0) {
                        toPrint = "100%";
                    }
                }
                ctx.fillText(toPrint, x - 38 - tooltipWidth, y - tooltipHeight / 2 + 36);
            }
        }
    }

    isChanged() {
        return this.xOld != this.x || this.yOld != this.y || this.aOld != this.a || this.wOld != this.w || this.hOld != this.h;
    }

    tick(game) {
        if (this.isChanged()) {
            this.box = this.bbox();
        }
        if (!this.didMove) {
            this.xOld = this.x;
            this.yOld = this.y;
            this.wOld = this.w;
            this.hOld = this.h;
            this.aOld = this.a;
        }
        this.didMove = false;
        if (this.goalPos.hasChanged) {
            this.goalPos.hasChanged = false;
            game.comms.move(this.id, this.goalPos.x, this.goalPos.y, this.goalPos.a);
        }
    }

    bbox() { // Produces a high-quality bbox of rotated rectangular objects
        var topleft = new Vector(-this.w / 2, -this.h / 2).rotate(this.a);
        var topright = new Vector(this.w / 2, -this.h / 2).rotate(this.a);
        var bottomleft = new Vector(-this.w / 2, this.h / 2).rotate(this.a);
        var bottomright = new Vector(this.w / 2, this.h / 2).rotate(this.a);
        var minX = topleft.sort((v1, v2) => {
            return v1.x < v2.x;
        }).with(topright).with(bottomleft).with(bottomright).x;
        var minY = topleft.sort((v1, v2) => {
            return v1.y < v2.y;
        }).with(topright).with(bottomleft).with(bottomright).y;
        var maxX = topleft.sort((v1, v2) => {
            return v1.x > v2.x;
        }).with(topright).with(bottomleft).with(bottomright).x;
        var maxY = topleft.sort((v1, v2) => {
            return v1.y > v2.y;
        }).with(topright).with(bottomleft).with(bottomright).y;
        return [this.x + minX, this.y + minY, this.x + maxX, this.y + maxY];
    }

    interact(game) {
        this.isOurs = game.mine.indexOf(this.id) != -1;
        if (!game.status.moveShips && !game.status.isRTF) {
            this.editState = 0;
            game.locked = false;
        }
        if (this.editState == 1) {
            this.goalPos.x = game.gameX;
            this.goalPos.y = game.gameY;
            this.goalPos.initialX = this.x;
            this.goalPos.initialY = this.y;
            var dx = this.goalPos.x - this.goalPos.initialX;
            var dy = this.goalPos.y - this.goalPos.initialY;
            dx *= dx;
            dy *= dy;
            this.goalPos.pathLen = Math.sqrt(dx + dy);
            this.goalPos.hasChanged = true;
        }
        else if (this.editState == 2) {
            this.goalPos.a = new Vector(game.gameX - this.goalPos.x, game.gameY - this.goalPos.y).angle();
            this.goalPos.hasChanged = true;
        }
    }

    click(game) { // called on EVERY CLICK, not just clicks where it's hovered
        if (!game.status.moveShips && !game.status.isRTF) {
            return;
        }
        if (this.editState == 1) {
            this.editState = 2;
        }
        else if (this.editState == 2) {
            this.editState = 0;
            game.locked = false;
        }
        if (!game.locked) {
            if (this.isHovered) {
                this.editState = 1;
                game.locked = true;
            }
        }
    }

    isCompassVisible() { // Can it be seen on a compass?
        const hidden = ["s", "b", "C", "w"]; // List of types that can't be displayed on minimap/compass
        var isSniper = this.upgrades.indexOf("s")!= -1;
        return this.isOurs || (hidden.indexOf(this.type) == -1 && !isSniper);
    }

    upgrade(upgrade) {
        this.upgrades.push(upgrade);
    }
}


class Game {
    constructor(socket) {
        this.comms = new Protocol(socket, (d1, d2) => {this.onmessage(d1, d2)});
        /*this.comms.addEventListener("protocolmessagereceived", (evt) => {
            this.onmessage(evt.command, evt.args);
        });*/
        this.gamesize = 0;
        this.zoomLevel = 0.7;
        this.canvas = document.getElementById("game");
        this.ctx = this.canvas.getContext("2d");
        this.health = -1;
        this.hasPlacedCastle = false;
        this.castle = undefined;
        this.ctx.makeRoundRect = function (x, y, width, height, rx, ry) { // Stolen from #platformer
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
        if (this.ctx.roundRect == undefined) { // Polyfill: roundRect doesn't have very good browser support (it's quite new: Chrome released support just last year and Firefox this year), so this is necessary. Keep your browsers up to date, dammit!
            this.ctx.roundRect = function (x, y, width, height, radii) {
                this.ctx.makeRoundRect(x, y, width, height, radii, radii);
            };
        }
        this.status = {
            spectating: false,
            counting: false,
            playing: false,
            online: false,
            moveShips: false,
            isTeamLeader: false,
            wallsRemaining: 4, // 4 on the first turn
            wallsTurn: 2, // 2 every next turn, but Extra Walls can increase this.
            wait: true,
            score: 0,
            countdown: 0, // Set by the ! command
            counter: 0, // Set by the t command
            tickTime: 1000 / 30, // Number of milliseconds between ticks or !s; this is adjusted based on real-time data.
            lastTickTime: -1,
            canPlaceObject: false, // if the mouse is close to the home castle with 400 tolerance
            mouseWithinNarrowField: false, // if the mouse is close to any game object with 400 tolerance
            mouseWithinWideField: false, // if the mouse is close to any game object with 600 tolerance
            isRTF: false,
            getTableBite() { // Don't ask
                return this.online ? (this.spectating ? "SPECTATING" : "ONLINE") : "OFFLINE";
            },
            getChairBites() { // Don't ask even more
                return this.wait ? "WAITING" : (this.moveShips ? "MOVE SHIPS" : "PLAY");
            },
            ticksRemaining() {
                return this.wait ? this.countdown : this.counter;
            },
            getTimes() {
                var _ms = this.ticksRemaining() * this.tickTime;
                var _secs = _ms / 1000;
                var minutes = Math.floor(_secs / 60);
                var seconds = Math.floor(_secs % 60);
                var hundreds = _secs % 1;
                hundreds = Math.round(hundreds * 100);
                return [minutes, seconds, hundreds];
            },
            getTimeString() {
                var times = this.getTimes();
                return "- " + zeroes("" + times[0]) + ":" + zeroes("" + times[1]) + ":" + zeroes("" + times[2]);
            }
        };
        this.mouseX = 0; // Mouse position in UI
        this.mouseY = 0;
        this.gameX = 0; // Mouse position in game
        this.gameY = 0;
        this.cX = 0; // Distance from the top-left corner of the actual gameboard to the center of the view screen
        this.cY = 0;
        this.sideScroll = 0;
        this.sidebar = new Sidebar();
        this.objects = {};
        this.banners = {}; // Relating banners to banner names
        this.teams = {}; // Relating player banners to team banners
        this.bgCall = undefined;
        this.mine = [];
        this.locked = false; // for objects to not be edited at the same time
        this.keysDown = {};
        if (DEBUG) {
            this.deletePoints = [];
        }
        this.controls = {
            up: false,
            left: false,
            down: false,
            right: false
        };
        this.inventory = [
            {
                name: "HYPERSONIC MISSILE",
                cost: 5,
                descriptionL1: "Very fast, very erratic missile that does",
                descriptionL2: "damage by crashing into enemies; does not shoot.",
                place: {
                    word: "h"
                }
            },
            {
                name: "ANTI-RTF SMART MISSILE",
                cost: 7,
                descriptionL1: "Very fast missile with clever kinematics that chases",
                descriptionL2: "down real time fighters and crashes into them.",
                place: {
                    word: "a"
                }
            },
            {
                name: "BASIC FIGHTER",
                cost: 10,
                descriptionL1: "Low motion speed, medium shot cooldown,",
                descriptionL2: "medium bullet range, 2 health.",
                place: {
                    word: "f"
                }
            },
            {
                name: "TIE FIGHTER",
                cost: 20,
                descriptionL1: "Slightly faster double barreled basic fighter.",
                descriptionL2: "Shoots out of back as well as front.",
                place: {
                    word: "t"
                }
            },
            {
                name: "SNIPER",
                cost: 30,
                descriptionL1: "Very fast low-profile cloaked fighter.",
                descriptionL2: "High shot cooldown, very high shot range.",
                place: {
                    word: "s"
                }
            },
            {
                name: "+2 WALL",
                cost: 30,
                descriptionL1: "Place 2 extra walls around any castle or fort",
                descriptionL2: "every turn.",
                place: {
                    cbk: () => {
                        this.status.wallsTurn += 2;
                    }
                }
            },
            {
                name: "TURRET",
                cost: 100,
                descriptionL1: "Stationary antiaircraft turret that swivels",
                descriptionL2: "towards enemy craft. Medium shot cooldown.",
                place: {
                    word: "T"
                }
            },
            {
                name: "MISSILE LAUNCHING SYSTEM",
                cost: 100,
                descriptionL1: "Stationary antiaircraft turret that swivels towards",
                descriptionL2: "enemy RTFs and fires heat-seaking missiles.",
                place: {
                    word: "m"
                }
            },
            {
                name: "FORT",
                cost: 120,
                descriptionL1: "Stationary, small low-profile fortress that you can place",
                descriptionL2: "fighters near. Can be placed anywhere. Backup castle.",
                place: {
                    word: "F"
                }
            },
            {
                name: "NUKE",
                cost: 300,
                descriptionL1: "Very fast well-controlled missile with a",
                descriptionL2: "high-yield nuclear warhead.",
                place: {
                    word: "n"
                }
            },
        ];
    }

    attemptWall(x, y) {
        if (this.status.canPlaceObject && this.status.wallsRemaining > 0) {
            this.place("w");
            this.status.wallsRemaining--;
        }
    }

    start(formdata) {
        var isSpectating = formdata.get("spectator") == "on";
        this.comms.connect(formdata.get("banner-name"), formdata.get("password-input"), formdata.get("playmode"), isSpectating);
    }

    onmessage(command, args) {
        if (command == "m") {
            this.gamesize = args[0] - 0;
            screen("gameui");
            this.bgCall = prerenderBackground(this.gamesize); // Pre-draw the background image onto a hidden canvas
            this._main();
        }
        else if (command == "t") {
            this.tick();
            this.status.wait = false; // If it's receiving TICK commands it is not waiting
            this.status.counter = args[0] - 0;
            if (!this.status.playing) {
                this.status.playing = true;
                notify("Game has started!")
            }
        }
        else if (command == "!") {
            this.status.countdown = args[0] - 0;
            this.status.wait = true;
            if (!this.status.counting) {
                this.status.counting = true;
                notify("Countdown started: Game will begin in " + this.status.getTimeString());
            }
        }
        else if (command == "?") {
            this.status.isTeamLeader = true;
        }
        else if (command == "B") {
            console.log("banner " + args[1] + " sent a message with priority " + args[2] + ": " + args[0]);
        }
        else if (command == "s") {

        }
        else if (command == "S") {
            this.status.score = args[0] - 0;
        }
        else if (command == "e") {
            
        }
        else if (command == "d") {
            if (this.castle && args[0] == this.castle.id) {
                delete this.castle;
                this.status.isRTF = false;
                this.status.spectating = true;
                this.mine = [];
            }
            if (DEBUG) {
                this.deletePoints.push([this.objects[args[0]].x, this.objects[args[0]].y]);
            }
            var index = this.mine.indexOf(args[0]);
            if (index != -1) {
                this.mine.splice(index, 1);
            }
            delete this.objects[args[0]];
        }
        else if (command == "n") {
            this.objects[args[1]] = new GameObject(args[2] - 0, args[3] - 0, args[7] - 0, args[8] - 0, args[4] - 0, args[0], args[5] == "1", args[1], args[6] - 0);
            if ((args[0] == "c" || args[0] == "R") && this.mine.indexOf(args[1]) != -1) {
                this.castle = this.objects[args[1]];
                if (args[0] == "R") {
                    this.status.isRTF = true;
                    this.inventory = [
                        {
                            name: "FASTER GUN",
                            cost: 30,
                            stack: 1,
                            descriptionL1: "Significantly decrease RTF main gun shot cooldown.",
                            descriptionL2: "",
                            place: {
                                upgrade: {
                                    effect: "castle",
                                    cost: 30,
                                    word: "b"
                                }
                            }
                        },
                        {
                            name: "SNIPER",
                            cost: 40,
                            stack: 1,
                            descriptionL1: "Make the RTF invisible on any scopes,",
                            descriptionL2: "including local-area compass.",
                            place: {
                                upgrade: {
                                    effect: "castle",
                                    word: "s"
                                }
                            }
                        },
                        {
                            name: "SPEEDSHIP",
                            cost: 70,
                            stack: 1,
                            descriptionL1: "Significantly increase RTF flight speed.",
                            descriptionL2: "",
                            place: {
                                upgrade: {
                                    effect: "castle",
                                    word: "f"
                                }
                            }
                        },
                        {
                            name: "FAST HEAL",
                            cost: 150,
                            stack: 1,
                            descriptionL1: "Significantly increase RTF repair speed.",
                            descriptionL2: "",
                            place: {
                                upgrade: {
                                    effect: "castle",
                                    word: "h"
                                }
                            }
                        },
                    ];
                }
            }
        }
        else if (command == "M") {
            var obj = this.objects[args[0]];
            if (!obj) {
                return;
            }
            obj.xOld = obj.x;
            obj.yOld = obj.y;
            obj.wOld = obj.w;
            obj.hOld = obj.h;
            obj.aOld = obj.a;
            this.ctx.fillStyle = "red";
            this.ctx.fillRect(obj.x, obj.y, 50, 50);
            obj.x = args[1] - 0;
            obj.y = args[2] - 0;
            if (args.length > 3) {
                obj.a = args[3] - 0;
            }
            if (args.length > 4) {
                obj.w = args[4] - 0;
                obj.h = args[5] - 0;
            }
            obj.didMove = true;
        }
        else if (command == "w") {
            if (args[0] == 0) {
                this.status.spectating = true;
            }
        }
        else if (command == "b") {
            this.banners[args[0]] = args[1];
            if (args.length > 2) {
                this.teams[args[1]] = args[2];
            }
        }
        else if (command == "a") {
            this.mine.push(args[0]);
        }
        else if (command == "l") {
            screen("youLose");
            setTimeout(() => {
                screen("gameui");
            }, 3000);
        }
        else if (command == "E") {
            if (this.castle.banner == args[0]) {
                screen("youWin");
            }
            else {
                document.getElementById("winnerBanner").innerText = this.banners[this.castle.banner];
                screen("gameOver");
            }
            setTimeout(() => {
                window.location.reload();
            }, 3000);
        }
        else if (command == "u") {
            this.objects[args[0]].upgrade(args[1]);
        }
        else {
            console.warn("UNRECOGNIZED COMMAND " + command + "!");
            console.log(args);
        }
        if (command == "t" || command == "!") {
            var oldStatus = this.status.moveShips;
            this.status.moveShips = args[1] == '1';
            if (this.status.moveShips && !oldStatus) {
                this.enterMoveShips();
            }
            var curTime = window.performance.now();
            if (this.status.lastTickTime == -1) {
                this.status.lastTickTime = curTime - this.status.tickTime;
            }
            var gTickTime = curTime - this.status.lastTickTime;
            const drag = 0.99;
            this.status.tickTime = this.status.tickTime * drag + gTickTime * (1 - drag);
            this.status.lastTickTime = curTime;
            if (args[2]) {
                this.health = args[2] - 0;
            }
        }
    }

    _main() {
        this.status.online = this.comms.isOnline;
        this.interactionLoop();
        this.renderLoop();
        if (!this.harikari) {
            requestAnimationFrame(() => { this._main() });
        }
    }

    renderGameboard(interpolator, zoomLevel = 0.3) {
        this.ctx.fillStyle = "#111111";
        this.ctx.save();
        if (this.bgCall) {
            this.bgCall(this.cX - window.innerWidth / 2, this.cY - window.innerHeight / 2);
        }
        this.ctx.drawImage(document.getElementById("background"), 0, 0); //offx, offy);
        this.ctx.translate(window.innerWidth / 2 - this.cX, window.innerHeight / 2 - this.cY);
        this.ctx.strokeStyle = "white";
        this.ctx.lineWidth = 4;
        this.ctx.strokeRect(0, 0, this.gamesize * zoomLevel, this.gamesize * zoomLevel);
        //this.ctx.fillRect(0, 0, this.gamesize, this.gamesize);
        Object.values(this.objects).forEach((item) => {
            item.draw(this, interpolator, zoomLevel);
        });
        if (DEBUG) {
            this.deletePoints.forEach(item => {
                this.ctx.beginPath();
                this.ctx.arc(item[0] * zoomLevel, item[1] * zoomLevel, 5, 0, Math.PI * 2);
                this.ctx.fill();
            });
        }
        this.ctx.fillStyle = "black";
        this.ctx.fillRect(this.gameX * zoomLevel - 5, this.gameY * zoomLevel - 5, 10, 10);
        this.ctx.restore();
    }

    cantPlace() { // COSMETIC: this is meant for user displays, NOT for logic.
        if (!this.status.hasPlacedCastle && this.status.mouseWithinNarrowField) { // if it isn't 
            return true;
        }
        if (this.sidebar.inventorySelected) {
            if (this.sidebar.inventorySelected.place.word == "F" && !this.status.mouseWithinNarrowField) {
                return false;
            }
            if (!this.status.canPlaceObject) {
                return true;
            }
        }
        return false;
    }

    drawStatus() {
        this.ctx.fillStyle = "#555555";
        this.ctx.font = "12px 'Chakra Petch'";
        this.ctx.textAlign = "left";
        this.ctx.fillText("SYSTEM STATUS", 18, 9 + 15.6 / 2);
        this.ctx.textAlign = "right";
        this.ctx.fillText("SCORE", window.innerWidth - 18, 9 + 15.6 / 2);
        this.ctx.font = "16px 'Chakra Petch'";
        this.ctx.fillStyle = "#CBCAFF";
        this.ctx.fillText(this.status.score, window.innerWidth - 18, 28 + 6 + 21 / 2);
        this.ctx.fillStyle = "white";
        this.ctx.textAlign = "left";
        var word = this.status.getTableBite();
        var width = this.ctx.measureText(word).width;
        this.ctx.fillRect(18, 28, width, 21);
        this.ctx.fillStyle = "black";
        this.ctx.fillText(word, 18, 28 + 6 + 21 / 2);
        word = this.status.getChairBites();
        this.ctx.fillStyle = "white";
        this.ctx.fillText(this.status.getChairBites(), 18 + width + 13, 28 + 6 + 21 / 2);
        width += 13 + this.ctx.measureText(word).width;
        this.ctx.fillStyle = "#CBCAFF";
        this.ctx.fillText(this.status.getTimeString(), 18 + width + 17, 28 + 6 + 21 / 2);
        if (this.cantPlace()) {
            this.ctx.fillStyle = "red";
            this.ctx.textAlign = "center";
            this.ctx.fillText("[ CAN'T PLACE HERE ]", window.innerWidth/2, 18);
        }
    }

    renderUI(interpolator) {
        this.ctx.translate(0, -this.sideScroll);
        //this.sidebar.isInventory = this.keysDown["i"] || this.sidebar.inventorySelected;
        this.sidebar.draw(this, interpolator);
        this.ctx.translate(0, this.sideScroll);
        this.ctx.fillStyle = "black";
        this.ctx.fillRect(0, 0, window.innerWidth, 56);
        this.ctx.fillRect(0, window.innerHeight - 56, window.innerWidth, 56);
        this.ctx.fillRect(0, 0, 9, window.innerHeight);
        this.ctx.fillRect(window.innerWidth - 56, 0, 56, window.innerHeight);
        this.ctx.beginPath();
        this.ctx.moveTo(window.innerWidth - 80, 56);
        this.ctx.quadraticCurveTo(window.innerWidth - 56, 56, window.innerWidth - 56, 80);
        this.ctx.lineTo(window.innerWidth - 56, 56);
        this.ctx.fill();
        this.ctx.moveTo(window.innerWidth - 80, window.innerHeight - 56);
        this.ctx.quadraticCurveTo(window.innerWidth - 56, window.innerHeight - 56, window.innerWidth - 56, window.innerHeight - 80);
        this.ctx.lineTo(window.innerWidth - 56, window.innerHeight - 56);
        this.ctx.fill();
        this.drawStatus();
    }

    renderLoop() { // Is called as much as possible; draws things and does smooth rendering
        this.ctx.fillStyle = "lightcoral";
        var interpolator = (window.performance.now() - this.status.lastTickTime) / this.status.tickTime;
        if (!INTERPOLATE) {
            interpolator = 1; // not 0, because then it'd be a frame behind at all times
        }
        if (this.status.isRTF && !this.status.moveShips && !this.status.wait) {
            this.cX = this.castle.getX(interpolator) * this.zoomLevel;
            this.cY = this.castle.getY(interpolator) * this.zoomLevel;
        }
        this.ctx.fillRect(0, 0, window.innerWidth, window.innerHeight);
        this.renderGameboard(interpolator, this.zoomLevel);
        this.renderUI(interpolator);
    }

    mouseFieldCheckOnOne(size, obj) {
        var bbox = obj.box;
        return this.gameX > bbox[0] - size && this.gameX < bbox[2] + size && this.gameY > bbox[1] - size && this.gameY < bbox[3] + size;
    }

    mouseFieldCheck(size) {
        var objects = Object.values(this.objects);
        for (var i = 0; i < objects.length; i++) {
            if (this.mouseFieldCheckOnOne(size, objects[i])) {
                return true;
            }
        }
        return false;
    }

    doMouse() {
        this.gameX = Math.round(clamp(0, Math.round(this.cX + this.mouseX - window.innerWidth / 2), this.gamesize)/this.zoomLevel);
        this.gameY = Math.round(clamp(0, Math.round(this.cY + this.mouseY - window.innerHeight / 2), this.gamesize)/this.zoomLevel);
        this.status.mouseWithinNarrowField = this.mouseFieldCheck(400);
        this.status.mouseWithinWideField = this.mouseFieldCheck(600);
        if (this.castle) {
            this.status.canPlaceObject = this.mouseFieldCheckOnOne(400, this.castle);
            this.mine.forEach(id => {
                var item = this.objects[id];
                if (item == undefined) {
                    return;
                }
                if (item.type == "F") {
                    this.status.canPlaceObject |= this.mouseFieldCheckOnOne(400, item);
                }
            });
            this.status.canPlaceObject &= this.status.moveShips || this.status.isRTF; // you can only place stuff during strat mode
        }
    }

    interactionLoop() { // Is called as much as possible; handles interaction with the user
        this.doMouse();
        Object.values(this.objects).forEach((item) => {
            item.bodyHovered = (this.gameX > item.x         - 10 && this.gameX < item.x         + 10 && this.gameY > item.y         - 10 && this.gameY < item.y         + 10)
            item.isHovered = item.bodyHovered ||
                               (this.gameX > item.goalPos.x - 5  && this.gameX < item.goalPos.x + 5  && this.gameY > item.goalPos.y - 5  && this.gameY < item.goalPos.y + 5);
            item.interact(this);
        });

        this.controls.up = this.keysDown["ArrowUp"] || this.keysDown["w"];
        this.controls.down = this.keysDown["ArrowDown"] || this.keysDown["s"];
        this.controls.left = this.keysDown["ArrowLeft"] || this.keysDown["a"];
        this.controls.right = this.keysDown["ArrowRight"] || this.keysDown["d"];
        if (!this.status.isRTF || this.status.moveShips) {
            if (this.controls.up) {
                this.cY -= 20;
            }
            else if (this.controls.down) {
                this.cY += 20;
            }
            if (this.controls.left) {
                this.cX -= 20;
            }
            else if (this.controls.right) {
                this.cX += 20;
            }
        }
        this.cX = clamp(0, this.cX, this.gamesize * this.zoomLevel);
        this.cY = clamp(0, this.cY, this.gamesize * this.zoomLevel);
        this.sideScroll = clamp(0, this.sideScroll, this.sidebar.scrollHeight - window.innerHeight + 56)
    }

    talk() { // Call every server tick; sends things to the server
        if (this.status.isRTF && !this.status.moveShips) {
            this.comms.rtf(this.controls.up, this.controls.left, this.controls.right, this.controls.down, this.keysDown[" "]);
        }
    }

    tick() { // Runs every server tick
        this.talk();
        Object.values(this.objects).forEach((item) => {
            item.tick(this);
        });
    }

    connectionClosed() {

    }

    scroll(dx, dy) {
        if (this.mouseX < 266) {
            this.sideScroll += dy + dx;
        }
        else {
            this.cX += dx;
            this.cY += dy;
            this.cX = clamp(0, this.cX, this.gamesize);
            this.cY = clamp(0, this.cY, this.gamesize);
        }
    }

    mouse(x, y) {
        this.mouseX = x;
        this.mouseY = y;
    }

    mouseDown() {
        
    }

    place(type) {
        this.comms.place(type, this.gameX, this.gameY);
    }

    mouseUp() {
        if (this.mouseX < 266) { // It's in the sidebar
            this.sidebar.clickies(this);
        }
        else {
            if (this.status.hasPlacedCastle) {
                if (this.sidebar.inventorySelected && (this.status.moveShips || this.status.isRTF)) {
                    if (this.status.canPlaceObject || (this.sidebar.inventorySelected.place.word == "F" && !this.status.mouseWithinNarrowField)) {
                        if (this.sidebar.inventorySelected.place.word) {
                            this.place(this.sidebar.inventorySelected.place.word);
                        }
                        if (this.sidebar.inventorySelected.stack) {
                            this.sidebar.inventorySelected.stack--;
                        }
                        this.sidebar.inventorySelected = undefined;
                        this.inventory.forEach(item => {
                            item.selected = false;
                        });
                    }
                }
                else {
                    Object.values(this.objects).forEach(item => {
                        if (item.isOurs) {
                            item.click(this);
                        }
                    });
                }
            }
            else if (!this.status.mouseWithinNarrowField){
                this.place("c");
                this.status.hasPlacedCastle = true;
            }
        }
    }

    kill() {
        this.harikari = true;
    }

    enterMoveShips() {
        this.status.wallsRemaining = this.status.wallsTurn;
    }
}


function screen(s) {
    Array.from(document.body.children).forEach(el => {
        if (el.id == s) {
            el.style.display = "";
        }
        else {
            el.style.display = "none";
        }
    });
}

screen("gameui");

var game = undefined;

function play() {
    randomizeBanner();
    var started = false;
    screen("establishin");
    var ws_url = document.getElementById("server-url").value;
    var socket = undefined;
    try {
        socket = new WebSocket(ws_url);
    }
    catch {
        screen("failedToConnect");
        setTimeout(() => {
            screen("startscreen")
        }, 1000);
        return;
    }
    game = new Game(socket);
    var form = new FormData(document.getElementById("startform"));
    socket.onopen = () => {
        started = true;
        game.start(form);
        window.addEventListener("wheel", (evt) => {
            game.scroll(evt.deltaX, evt.deltaY);
            evt.preventDefault();
            return false;
        }, {passive:false});

        window.addEventListener("scroll", (evt) => {
            evt.preventDefault();
            return false;
        });

        window.addEventListener("mousemove", (evt) => {
            game.mouse(evt.clientX, evt.clientY);
        });

        window.addEventListener("mousedown", (evt) => {
            game.mouseDown();
        });

        window.addEventListener("mouseup", (evt) => {
            game.mouseUp();
        });

        window.addEventListener("keydown", (evt) => {
            game.keysDown[evt.key] = true;
            if (evt.key == "q") {
                game.attemptWall(game.mouseX, game.mouseY);
            }
        });
        
        window.addEventListener("keyup", (evt) => {
            game.keysDown[evt.key] = false;
            if (evt.key == "i") {
                game.sidebar.isInventory = !game.sidebar.isInventory;
            }
        });
    };
    socket.onerror = () => {
        if (!started) {
            screen("failedToConnect");
            setTimeout(() => {
                screen("startscreen");
            }, 1000);
        }
    };
    socket.onclose = () => {
        game.connectionClosed();
    };
}

function resizah() {
    var game = document.getElementById("game");
    game.width = window.innerWidth;
    game.height = window.innerHeight;
    var background = document.getElementById("background");
    if (diva) {
        background.width = divaW;
        background.height = divaH;
        background.style.display = "";
    }
    else{
        background.width = window.innerWidth;
        background.height = window.innerHeight;
    }
};

resizah();

window.addEventListener("resize", resizah);

function zeroes(num, size = 2) { 
    while (num.length < size) {
        num = "0" + num;
    }
    return num;
}

function guessWS() {
    var construct = "ws";
    if (window.location.protocol == "https:") {
        construct += "s";
    }
    construct += "://";
    construct += window.location.host + window.location.pathname;
    construct += "/game";
    return construct;
}


/* OPTIMIZATION:
    In the olden days, the client was constantly sending requests (usually MOVE) at every frame, or thereabouts!
    This is obviously unsuitable because my system updates at 120 hz, most systems update at 60hz, and the server updates at 30hz.
    *At best, the server was receiving updates twice as often as optimal*. This is bad, but it's even worse when you consider RTFs:
    they have to send a relatively sizeable broadcast at whatever framerate the system uses *just to be usable at all*.
    In effect, every game we played was ddosing my server.

    Gonna try to avoid that!
*/

randomizeBanner();

if (Notification.permission == "default") {
    screen("allownotifs");
}
else {
    screen("startscreen");
}

function allowNotifs() {
    Notification.requestPermission().then(() => {
        screen("startscreen");
    });
}