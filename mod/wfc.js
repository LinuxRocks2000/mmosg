/*
    WaveFunction Collapse Terrain Generator
    For simplicity this "learns" and generates with large ASCII terrain maps with canvas commands
    possibilities For Now : /, \, |, _
*/

const trainingmaterial = String.raw`   _
 _/ \
/ /\ \
| \/ /
|  _ |
| | \ \
| |  \ \_
| |   \  \
| \   / _/
 \ \_/ /
  \___/`
    ;

function getNxNs(map, n) { // get every nxn square in the map
    map = map.split("\n");
    var ret = [];
    for (var x = 0; x < map.length - n + 1; x++){
        for (var y = 0; y < map[x].length - n + 1; y++){
            var nxn = [];
            for (var i = 0; i < n; i++) {
                nxn.push(map[x + i].substring(y, y + n));
            }
            ret.push(nxn);
        }
    }
    return ret;
}

function mkSuperposition(width, height, values) {
    var ret = [];
    for (var x = 0; x < width; x++) {
        ret.push([]);
        for (var y = 0; y < height; y++) {
            ret[x].push(Array.from(values)); // copy and push
        }
    }
    return ret;
}

function entropyCheck(superposition) { // Return a list of lowest-entropy x,ys in a superposition
    var ret = [];
    var standingEntropy = Infinity;
    for (var x = 0; x < superposition.length; x++) {
        for (var y = 0; y < superposition[x].length; y++) {
            var entropy = superposition[x][y].length;
            if (entropy > 1) { // Only pickable squares, ergo squares that haven't already been solved
                if (entropy < standingEntropy) {
                    ret = [[x, y]];
                    standingEntropy = entropy;
                }
                else if (entropy == standingEntropy) {
                    ret.push([x, y]);
                }
            }
        }
    }
    return ret;
}

function pickPos(superposition) {
    var check = entropyCheck(superposition);
    return check[Math.floor(check.length * Math.random())];
}

function ruleMatch(superposition, x, y, rule) {
    
}

function getPossibles(superposition, x, y, rules) {
    var matches = [];
    for (var i = 0; i < rules.length; i++) {
        matches.push(...ruleMatch(superposition, x, y, rules[i]));
    }
    return matches;
}

function propagate(superposition, rules) {
    for (var x = 0; x < superposition.length; x++) {
        for (var y = 0; y < superposition[x].length; y++) {
            superposition[x][y] = getPossibles(superposition, x, y, rules);
        }
    }
}

function collapseRandom(superposition, rules) {
    var pos = pickPos(superposition);
    var posse = superposition[pos[0]][pos[1]];
    while (posse.length > 1) { // Lower the length to a single possibility in a way that will preserve references
        posse.splice(Math.floor(Math.random() * posse.length), 1);
    }
    propagate(superposition, rules);
}

var superposition = mkSuperposition(30, 30, [" ", "/", "\\", "_", "|"]); // Terrain map superposition
var rules = getNxNs(trainingmaterial, 2); // The list of possible nxn squares in the grid

collapseRandom(superPos, nxns);

function prerenderBackground(size) {
    // TODO
}