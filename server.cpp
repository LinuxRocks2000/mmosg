/* By Tyler Clarke
    The server for MMOSG, a multiplayer strategy game (basically Rampart + Galaga).
    Please give credit if you copy or otherwise use anything in here!

    The structure:
    FPS times per second, the game is simulated another tick.
    Every time something changes, all the connected Clients are sent some information on what happened, such as
*/

#define PI 3.141592

#define CROW_ENFORCE_WS_SPEC
#include <crow.h>
#include <stdio.h>
#include <sys/time.h>
#include <unistd.h>
#include <pthread.h>
#include <vector>
#include <mutex>
#include <functional>
#include "logo.h"
#include "terminalstuff.hpp"
#include <signal.h>
#include "vector.hpp"
#define FPS 30
const float ALLOWED_MICROS_PER_FRAME = 1000000.f/FPS;
char* code;

unsigned int counter = 1;
unsigned int livePlayerCount = 0;
bool stratChangeMode = false;
bool playing = false;
unsigned long gameSize = 5000;
long millisPerTick = 0;
std::string serverName = "StrategyGameMMO";
CleverTerminal terminal;

bool isAutonomous = false;
int autonomousMaxPlayers = 0;
int autonomousMinPlayers = 0;
int autonomousStartTimer = 0;
int autonomousTimer = 0;
bool teamMode = false;

unsigned int terrainSeed;

std::vector<std::string> banners;

long clamp (long min, long val, long max){
    if (val < min){
        val = min;
    }
    if (val > max){
        val = max;
    }
    return val;
}

struct team {
    long TLbanner = -1;
    size_t banner;
    char* password;
    bool refPassed = false;
    ~team(){
        if (!refPassed){
            free(password);
        }
    }

    team(team& t){
        t.refPassed = true;
        password = t.password;
        banner = t.banner;
    }

    team(size_t bann, char* passwd) {
        banner = bann;
        password = passwd;
    }
};

std::vector<team*> teams;

void broadcast(std::string broadcast);

size_t getTeamOf(size_t banner);


void addBanner(std::string thing, size_t team = 0){
    banners.push_back(thing);
    broadcast((std::string)"b" + std::to_string(banners.size() - 1) + " " + thing + (teamMode ? " " + std::to_string(teams[team] -> banner) : "")); // b = add banner
}


template <int length = 32>
char* randCode(){
    const char* characters = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!@#&";
    const int len = strlen(characters);
    char* ret = (char*)malloc(length);
    for (int x = 0; x < length; x ++){
        ret[x] = characters[rand() % len];
    }
    return ret;
}


double coterminal(double angle, double about = PI * 2){
    while (angle < 0){
        angle += about;
    }
    while (angle >= about){
        angle -= about;
    }
    return angle;
}


double errorify(double one, double two, double about = PI * 2) {
    one = coterminal(one, about);
    two = coterminal(two, about);
    double error = coterminal(two - one, about);
    if (error < PI){
        error = error - PI * 2;
    }
    return error;
}


std::vector<std::string> splitString(std::string str, char delim){
    std::vector<std::string> ret;
    std::string buf;
    bool escape;
    for (char c : str){
        if ((c == delim) && (!escape)){
            ret.push_back(buf);
            buf = "";
        }
        else if ((c == '\\') && (!escape)){
            escape = true;
        }
        else{
            buf += c;
            escape = false;
        }
    }
    if (buf.size() != 0){
        ret.push_back(buf);
    }
    return ret;
}


struct Box {
    double x1;
    double y1;
    double x2;
    double y2;

    bool check(Box other){
        return (other.x1 < x2) && (other.x2 > x1) && (other.y1 < y2) && (other.y2 > y1);
    }
};


bool listContains(std::vector<auto> list, auto item) {
    for (auto& thing : list){
        if (thing == item){
            return true;
        }
    }
    return false;
}


struct Client;


struct GameObject {
    double health = 1;
    double damage = 1;

    long banner = -1;

    static unsigned long topId;

    unsigned long id = -1;
    double x = 0;
    double y = 0;
    float angle = 0;

    long goalX = 0;
    long goalY = 0;
    float goalAngle = 0;
    bool rm = false;

    Client* owner = NULL;
    GameObject* killer = NULL;

    std::function <void(GameObject*)> lostFunction;
    bool hasLostCallback = false;

    void setLostCallback(std::function<void(GameObject*)> function){
        lostFunction = function;
        hasLostCallback = true;
    }

    virtual ~GameObject () {
        broadcast((std::string)"d" + std::to_string(id));
    }

    virtual void update() {

    }

    virtual Box box () {
        return { 0, 0, 0, 0 };
    }

    virtual char identify() {
        return 'g';
    }

    virtual bool editable() { // All things have the same properties: position and rotation. Not one or the other, both or neither.
        return false;
    }

    virtual void destroy() {
        rm = true;
        if (hasLostCallback){
            lostFunction(this);
        }
    }

    virtual void greet() {
        // Extend to "greet" new clients
    }

    void shoot(float angle, float speed = 20, float dist = 50, int duration = -1);
};


std::vector <GameObject*> objects;


class FortObject : public GameObject {
public:
    void destroy() {
        rm = true;
    }

    void update() {

    }

    Box box(){
        return { x - 10, y - 10, x + 10, y + 10 };
    }

    char identify() {
        return 'F';
    }
};


class CastleObject : public GameObject {
public:
    vector velocity;
    bool isRTF = false;
    float angularVelocity = 0;
    long shootCycle = 0;

    CastleObject(){
        health = 3;
    }

    void enableRTF(){
        health = 3;
        isRTF = true;
    }

    std::vector<GameObject*> forts;
    void update() {
        shootCycle --;
        for (size_t f = 0; f < forts.size(); f ++){
            if (forts[f] -> rm){
                forts.erase(forts.begin() + f);
                f --;
            }
        }
        if (isRTF){
            if (velocity.magnitude() > 20){
                velocity.setMagnitude(20);
            }
            x += velocity.x;
            y += velocity.y;
            x = coterminal(x, gameSize);
            y = coterminal(y, gameSize);
            angle += angularVelocity;
            angularVelocity *= 0.9;
            broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle + PI/2));
        }
        
    }

    void rtf(bool forwards, bool left, bool right, bool airbrake, bool shewt){
        if (airbrake){
            velocity.setMagnitude(velocity.magnitude() * 0.8);
        }
        vector thrust;
        thrust.setMandA(forwards * 2, angle);
        velocity += thrust;
        angularVelocity += (left * -1 + right) * 0.02;
        if (shewt){
            if (shootCycle < 0){
                shootCycle = 15;
                shoot(angle, 35, 50);
            }
        }
    }

    void destroy () {
        if (forts.size() > 0) {
            x = forts[0] -> x;
            y = forts[0] -> y;
            forts[0] -> rm = true;
            forts.erase(forts.begin());
            broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
        }
        else {
            rm = true;
            if (hasLostCallback){
                lostFunction(this);
            }
        }
    }

    Box box(){
        return { x - 25, y - 25, x + 25, y + 25 };
    }

    char identify(){
        return isRTF ? 'R' : 'c';
    }
};


class BasicFighterObject : public GameObject {
    float xv = 0;
    float yv = 0;

    int shootTimer = 20;

    void update() {
        shootTimer --;
        if (shootTimer < 0){
            shootTimer = 30;
            shoot(angle);
        }
        double dx = goalX - x;
        double dy = goalY - y;
        if (((dx * dx) + (dy * dy)) > 10 * 10){
            angle = atan(dy/dx);
            if (dx <= 0){
                angle += PI;
            }
            xv += cos(angle) * 0.25;
            yv += sin(angle) * 0.25;
        }
        else {
            angle = goalAngle;
        }
        x += xv;
        y += yv;
        xv *= 0.95;
        yv *= 0.95;
        broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
    }

    char identify() {
        return 'f';
    }

    bool editable() {
        return true;
    }

    Box box (){
        return { x - 10, y - 10, x + 10, y + 10 };
    }
};


class HypersonicMissileObject : public GameObject {
    float xv = 0;
    float yv = 0;

    void update() {
        double dx = goalX - x;
        double dy = goalY - y;
        float goToAngle = goalAngle;
        if (((dx * dx) + (dy * dy)) > 10 * 10){
            goToAngle = atan(dy/dx);
            if (dx <= 0){
                goToAngle += PI;
            }
        }
        angle = angle * 0.9 + goToAngle * 0.1;
        xv += cos(angle) * 0.3;
        yv += sin(angle) * 0.3;
        xv *= 0.99;
        yv *= 0.99;
        x += xv;
        y += yv;
        broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
    }

    char identify() {
        return 'h';
    }

    bool editable() {
        return true;
    }

    Box box (){
        return { x - 3, y - 3, x + 3, y + 3 };
    }
};


class TieFighterObject : public GameObject {
    float xv = 0;
    float yv = 0;

    int shootTimer = 30;

    void update() {
        shootTimer --;
        if (shootTimer < 0){
            shootTimer = 40;
            shoot(angle);
            shoot(angle + PI);
        }
        double dx = goalX - x;
        double dy = goalY - y;
        if (((dx * dx) + (dy * dy)) > 10 * 10){
            angle = atan(dy/dx);
            if (dx <= 0){
                angle += PI;
            }
            xv += cos(angle) * 0.35;
            yv += sin(angle) * 0.35;
        }
        else {
            angle = goalAngle;
        }
        x += xv;
        y += yv;
        xv *= 0.95;
        yv *= 0.95;
        broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
    }

    char identify() {
        return 't';
    }

    bool editable() {
        return true;
    }

    Box box (){
        return { x - 10, y - 10, x + 10, y + 10 };
    }
};


class SniperObject : public GameObject {
    float xv = 0;
    float yv = 0;

    int shootTimer = 60;

    void update() {
        shootTimer --;
        if (shootTimer < 0){
            shootTimer = 80; // They shoot far less often than the other guys
            shoot(angle, 25, 20, 90); // 5 faster than regular bullets, and they last 3 times longer (they're snipers)
        }
        double dx = goalX - x;
        double dy = goalY - y;
        if (((dx * dx) + (dy * dy)) > 10 * 10){
            angle = atan(dy/dx);
            if (dx <= 0){
                angle += PI;
            }
            xv += cos(angle) * 1.2;
            yv += sin(angle) * 1.2;
        }
        else {
            angle = goalAngle;
        }
        x += xv;
        y += yv;
        xv *= 0.9;
        yv *= 0.9;
        broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
    }

    char identify() {
        return 's';
    }

    bool editable() {
        return true;
    }

    Box box (){
        return { x - 10, y - 10, x + 10, y + 10 };
    }
};


class BulletObject : public GameObject {
public:
    float xv = 0;
    float yv = 0;

    int TTL = 30; // they last the same number of ticks, even if you speed up the framerate

    char identify(){
        return 'b';
    }

    void update() {
        TTL --;
        if (TTL < 0){
            rm = true;
        }
        y += yv;
        x += xv;
        broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
    }

    Box box () {
        return { x - 5, y - 5, x + 5, y + 5 };
    }
};


class RadiationObject : public GameObject {
public:
    int halflife = 150;
    double startingStrength = 0.5;
    unsigned long cycle = 0;
    Box mBox { -200, -200, 200, 200 };

    RadiationObject() {
        health = 10000000;
    }

    char identify(){
        return 'r';
    }

    void update() {
        double strength = std::pow(0.5, ((double)cycle)/halflife) * startingStrength;
        broadcast("r" + std::to_string(id) + " " + std::to_string(strength));
        cycle ++;
        if (strength < 0.01){
            rm = true;
        }
        damage = strength/15;
    }

    void destroy(){
        // do nothing! Radiation can't be killed
    }

    Box box () {
        return { x + mBox.x1, y + mBox.y1, x + mBox.x2, y + mBox.y2 };
    }

    void upd8Box () {
        broadcast("█" + std::to_string(id) + " " + std::to_string(mBox.x1) + " " + std::to_string(mBox.y1) + " " + std::to_string(mBox.x2) + " " + std::to_string(mBox.y2));
    }

    void greet() {
        upd8Box();
    }
};


class NukeObject : public GameObject {
public:
    ~NukeObject(){
        broadcast((std::string)"d" + std::to_string(id));
        blowUP();
    }

    float xv = 0;
    float yv = 0;

    int explodeTimer = 500;

    void blowUP();

    void update() {
        explodeTimer --;
        if (explodeTimer < 0){
            destroy();
        }
        double dx = goalX - x;
        double dy = goalY - y;
        if (((dx * dx) + (dy * dy)) > 10 * 10){
            angle = atan(dy/dx);
            if (dx <= 0){
                angle += PI;
            }
            xv += cos(angle) * 0.1;
            yv += sin(angle) * 0.1;
        }
        else {
            angle = goalAngle;
        }
        x += xv;
        y += yv;
        xv *= 0.999;
        yv *= 0.999;
        broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
    }

    void destroy() {
        terminal.printLn("destroying");
        broadcast("j100");
        if (hasLostCallback){
            lostFunction(this);
        }
        rm = true;
    }

    char identify() {
        return 'n';
    }

    bool editable() {
        return true;
    }

    Box box (){
        return { x - 10, y - 10, x + 10, y + 10 };
    }
};


class WallObject : public GameObject {
public:
    WallObject() {
        health = 2;
    }
    char identify() {
        return 'w';
    }

    int TTL = 600;

    void update() {
        TTL --;
        if (TTL < 0){
            rm = true;
        }
    }

    Box box () {
        return { x - 15, y - 15, x + 15, y + 15 };
    }
};


class TurretObject : public GameObject {
    double angleV = 0;
    int shootTimer = 20;

    char identify() {
        return 'T';
    }

    void update() {
        GameObject* closestShootah = NULL;
        double bestDistance = 500 * 500; // It won't seek ships more than 500 away
        double bestDx = 0;
        double bestDy = 0;
        for (GameObject* obj : objects){
            if (obj -> owner != owner){
                char id = obj -> identify();
                if ((id != 'b') && (id != 'T') && (id != 'w') && (id != 'c') && (id != 'C')){
                    double dX = obj -> x - x;
                    double dY = obj -> y - y;
                    double d = dX * dX + dY * dY;
                    if (d < bestDistance){
                        bestDistance = d;
                        closestShootah = obj;
                        bestDx = dX;
                        bestDy = dY;
                    }
            }
            }
        }
        if (closestShootah != NULL){
            goalAngle = atan2(bestDy, bestDx);
            shootTimer --;
            if (shootTimer < 0){
                shoot(angle);
                shootTimer = 30;
            }
        }
        angleV += errorify(angle, goalAngle) * -0.005;
        angleV *= 0.85;
        angle += angleV;
        broadcast((std::string)"M" + std::to_string(id) + " " + std::to_string(x) + " " + std::to_string(y) + " " + std::to_string(angle));
    }

    Box box (){
        return { x - 10, y - 10, x + 10, y + 10 };
    }
};


void addObject(GameObject* object);


struct Client {
    crow::websocket::connection& conn;
    std::mutex sendMutex;
    std::mutex processingMutex; // For generic INTERNAL tasks
    bool is_authorized = false;
    bool hasPlacedCastle = false;
    CastleObject* deMoi = new CastleObject;
    std::vector <GameObject*> myFighters;
    long score = 0;
    size_t myBanner = -1;
    size_t myTeam = -1;
    char arrangement = 'k';

    void collect(int amount){
        score += amount;
        sendText("S" + std::to_string(score));
    }

    ~Client (){
        for (GameObject* obj : myFighters){
            obj -> owner = NULL;
            obj -> hasLostCallback = false;
        }
        if (is_authorized){
            livePlayerCount --;
        }
    }

    void sendText(std::string text){
        sendMutex.lock();
        conn.send_text(text);
        sendMutex.unlock();
    }

    void process(std::string message){ // INSIDE A CROW THREAD
        if (message == "_"){ // ping
            sendText("_"); // pong
            return;
        }
        char command = message[0];
        std::vector<std::string> args = splitString(message.substr(1, message.size() - 1), ' ');
        processingMutex.lock();
        if (command == 'c'){
            if ((!isAutonomous || (livePlayerCount < autonomousMaxPlayers )) && (args[0] != "_spectator") && (!playing)){ // If it's not autonomous OR the current player count is under max AND the first argument (code) is not spectator mode, and we aren't playing.
                while (listContains(banners, args[1])){
                    args[1] += ".copy";
                }
                if (teamMode){
                    team* theTeam = NULL;
                    for (team* t : teams){
                        if (t -> password == args[0]) {
                            theTeam = t;
                            break;
                        }
                    }
                    if (theTeam) {
                        terminal.printLn("New player logged in with valid team " + banners[theTeam -> banner] + " access code.");
                        sendText("s");
                        is_authorized = true;
                        livePlayerCount ++;
                        addBanner(args[1], theTeam -> banner);
                        myTeam = theTeam -> banner;
                        myBanner = banners.size() - 1;
                        if (theTeam -> TLbanner == -1){
                            theTeam -> TLbanner = myBanner;
                            sendText("s1");
                        }
                        metadata();
                        terminal.printLn("New banner " + args[1]);
                    }
                    else {
                        terminal.printLn("Player failed to log in - has the wrong code?");
                        sendText("e0"); // ERROR 0 invalid code
                    }
                }
                else {
                    if (args[0] == code){
                        terminal.printLn("New player logged in with access code!");
                        sendText("s"); // SUCCESS
                        is_authorized = true;
                        livePlayerCount ++;
                        addBanner(args[1]);
                        myBanner = banners.size() - 1;
                        metadata();
                        terminal.printLn("New banner " + args[1]);
                    }
                    else{
                        terminal.printLn("Player failed to log in - has the wrong code?");
                        sendText("e0"); // ERROR 0 invalid code
                    }
                }
                arrangement = args[2][0];
                if (args[2] == "p"){
                    deMoi -> enableRTF();
                }
            }
            else{
                terminal.printLn("A spectator entered the arena!");
                sendText("w0"); // WARNING 0 you are a spectator
                metadata();
            }
        }
        else if (is_authorized){
            if (command == 'p'){
                if (args[0] == "c"){
                    if (!hasPlacedCastle){
                        hasPlacedCastle = true;
                        deMoi -> x = std::stoi(args[1]);
                        deMoi -> y = std::stoi(args[2]);
                        add(deMoi);
                        collect(50);
                        fighters();
                    }
                    else{
                        terminal.printLn("Some idiot just tried to hack this system");
                    }
                }
                else if (args[0] == "w"){
                    WallObject* w = new WallObject;
                    w -> x = std::stoi(args[1]);
                    w -> y = std::stoi(args[2]);
                    add(w);
                }
                else if (args[0] == "f"){
                    if (score >= 10){
                        BasicFighterObject* f = new BasicFighterObject;
                        f -> x = std::stoi(args[1]);
                        f -> y = std::stoi(args[2]);
                        f -> goalX = f -> x;
                        f -> goalY = f -> y;
                        add(f);
                        collect(-10);
                    }
                }
                else if (args[0] == "t"){
                    if (score >= 20){
                        TieFighterObject* f = new TieFighterObject;
                        f -> x = std::stoi(args[1]);
                        f -> y = std::stoi(args[2]);
                        f -> goalX = f -> x;
                        f -> goalY = f -> y;
                        add(f);
                        collect(-20);
                    }
                }
                else if (args[0] == "s"){
                    if (score >= 30){
                        SniperObject* f = new SniperObject;
                        f -> x = std::stoi(args[1]);
                        f -> y = std::stoi(args[2]);
                        f -> goalX = f -> x;
                        f -> goalY = f -> y;
                        add(f);
                        collect(-30);
                    }
                }
                else if (args[0] == "h"){
                    if (score >= 5){
                        HypersonicMissileObject* f = new HypersonicMissileObject;
                        f -> x = std::stoi(args[1]);
                        f -> y = std::stoi(args[2]);
                        f -> goalX = f -> x;
                        f -> goalY = f -> y;
                        add(f);
                        collect(-5);
                    }
                }
                else if (args[0] == "n"){
                    if (score >= 150){
                        NukeObject* f = new NukeObject;
                        f -> x = std::stoi(args[1]);
                        f -> y = std::stoi(args[2]);
                        f -> goalX = f -> x;
                        f -> goalY = f -> y;
                        add(f);
                        collect(-150);
                    }
                }
                else if (args[0] == "T"){
                    if (score >= 100){
                        TurretObject* f = new TurretObject;
                        f -> x = std::stoi(args[1]);
                        f -> y = std::stoi(args[2]);
                        f -> goalX = f -> x;
                        f -> goalY = f -> y;
                        add(f);
                        collect(-100);
                    }
                }
                else if (args[0] == "F"){
                    if (score >= 120){
                        FortObject* f = new FortObject;
                        f -> x = std::stoi(args[1]);
                        f -> y = std::stoi(args[2]);
                        f -> goalX = f -> x;
                        f -> goalY = f -> y;
                        add(f);
                        collect(-120);
                    }
                }
            }
            else if (command == 'm'){
                for (GameObject* obj : myFighters){
                    if (obj -> id == std::stoi(args[0])) {
                        obj -> goalX = std::stoi(args[1]);
                        obj -> goalY = std::stoi(args[2]);
                        obj -> goalAngle = std::stod(args[3]);
                    }
                }
            }
            else if (command == 'C'){
                long monies = std::abs(std::stoi(args[0]));
                collect(-monies);
            }
            else if (command == 'R') {
                if (deMoi -> isRTF){
                    deMoi -> rtf(args[0][0] == '1', args[0][1] == '1', args[0][2] == '1', args[0][3] == '1', args[0][4] == '1');
                }
            }
        }
        else{
            sendText("e1"); // ERROR 1 premature command
        }
        processingMutex.unlock();
    }

    void metadata(){
        sendText("m" + std::to_string(gameSize) + " " + std::to_string(terrainSeed));
        for (GameObject* obj : objects){
            sendObject(obj);
            obj -> greet();
        }
        for (size_t banner = 0; banner < banners.size(); banner ++){
            size_t t = getTeamOf(banner);
            if (t != -1){
                sendText((std::string)"b" + std::to_string(banner) + " " + banners[banner] + " " + std::to_string(teams[t] -> banner));
            }
            else {
                sendText((std::string)"b" + std::to_string(banner) + " " + banners[banner]);
            }
        }
    }

    void sendObject(GameObject* obj){
        sendText((std::string)"n" + obj -> identify() + " " + std::to_string(obj -> id) + " " + std::to_string(obj -> x) + " " + std::to_string(obj -> y) + " " + std::to_string(obj -> angle) + " " + (obj -> editable() ? "1" : "0") + " " + std::to_string(obj -> banner));
    }

    void tick(unsigned int counter, bool mode){
        sendText((std::string)"t" + std::to_string(counter) + " " + (mode ? "1" : "0"));
    }

    void lostCallback(GameObject* thing){
        long res = -1;
        for (size_t i = 0; i < myFighters.size(); i ++){
            if (myFighters[i] -> id == thing -> id){
                res = i;
            }
        }
        if (res >= 0){
            myFighters.erase(myFighters.begin() + res);
            switch (thing -> identify()){
                case 'R':
                case 'c':
                    if (thing -> killer != NULL && thing -> killer -> owner != NULL){
                        if (thing -> killer -> owner != this){
                            thing -> killer -> owner -> collect(50); // Enemy castles are worth 50 points
                        }
                    }
                    sendText((std::string)"l");
                    is_authorized = false;
                    livePlayerCount --;
                    break;
                case 'f':
                    if (thing -> killer != NULL && thing -> killer -> owner != NULL){
                        if (thing -> killer -> owner != this){
                            thing -> killer -> owner -> collect(15); // You get 15 points for taking out an enemy fighter.
                        }
                    }
                    break;
                case 'w':
                    if (thing -> owner != NULL){
                        thing -> owner -> collect(-2); // You lose 2 points for losing a wall - nobody gains anything, though
                    }
                    break;
                case 't':
                    if (thing -> killer != NULL && thing -> killer -> owner != NULL){
                        if (thing -> killer -> owner != this){
                            thing -> killer -> owner -> collect(30); // You get 15 points for taking out an enemy tiefighter
                        }
                    }
                    break;
                case 's':
                    if (thing -> killer != NULL && thing -> killer -> owner != NULL){
                        if (thing -> killer -> owner != this){
                            thing -> killer -> owner -> collect(45); // You get 45 points for taking out an enemy sniper
                        }
                    }
                    break;
                case 'h':
                    if (thing -> killer != NULL && thing -> killer -> owner != NULL){
                        if (thing -> killer -> owner != this){
                            thing -> killer -> owner -> collect(7);
                        }
                    }
                    break;
                case 'T':
                    if (thing -> killer != NULL && thing -> killer -> owner != NULL){
                        if (thing -> killer -> owner != this){
                            thing -> killer -> owner -> collect(50);
                        }
                    }
                    break;
                case 'F':
                    if (thing -> killer != NULL && thing -> killer -> owner != NULL){
                        if (thing -> killer -> owner != this){
                            thing -> killer -> owner -> collect(20);
                        }
                        if (thing -> owner){
                            thing -> owner -> collect(-10);
                        }
                    }
                    break;
            }
        }
    }

    void add(GameObject* thing){
        thing -> owner = this;
        thing -> banner = myBanner;
        thing -> setLostCallback([this](GameObject* thing){
            this -> lostCallback(thing);
        });
        myFighters.push_back(thing);
        addObject(thing);
        sendText((std::string)"a" + std::to_string(thing -> id));
        if (thing -> identify() == 'F'){
            deMoi -> forts.push_back(thing);
        }
    }

    template <typename FighterType>
    FighterType* newFighter(unsigned long x, unsigned long y, float angle){
        FighterType* thing = new FighterType;
        thing -> x = x;
        thing -> y = y;
        thing -> goalX = x;
        thing -> goalY = y;
        thing -> angle = angle;
        thing -> goalAngle = angle;
        add(thing);
        return thing;
    }

    template <typename T>
    T* newRelativeThing(long relX, long relY, float angle = 0){
        return newFighter<T>(deMoi -> x + relX, deMoi -> y + relY, angle);
    }

    void newRelativeFighter(long relX, long relY, float angle = 0){
        newFighter<BasicFighterObject>(deMoi -> x + relX, deMoi -> y + relY, angle);
    }

    void newRelativeSniper(long relX, long relY, float angle = 0){
        newFighter<SniperObject>(deMoi -> x + relX, deMoi -> y + relY, angle);
    }

    void newRelativeTiefighter(long relX, long relY, float angle = 0){
        newFighter<TieFighterObject>(deMoi -> x + relX, deMoi -> y + relY, angle);
    }

    void newRelativeHypersonicMissile(long relX, long relY, float angle = 0){
        newFighter<HypersonicMissileObject>(deMoi -> x + relX, deMoi -> y + relY, angle);
    }

    void fighters(){
        switch (arrangement) {
            case 'k':
                newRelativeFighter(200, 0);
                newRelativeFighter(-200, 0, PI);
                newRelativeFighter(0, 200);
                newRelativeFighter(0, -200);
                break;
            case 'p':
                newRelativeFighter(100, 0, PI/2);
                newRelativeFighter(-100, 0, PI/2);
                break;
            case 'd':
                newRelativeFighter(200, 0);
                newRelativeFighter(-200, 0, PI);
                newRelativeThing<TurretObject>(0, -200);
                newRelativeThing<TurretObject>(0, 200);
                break;
        }
        //newRelativeSniper(100, 0, PI/2);
        //newRelativeTiefighter(0, 100);
        //newRelativeTiefighter(0, -100, PI);
        //newRelativeHypersonicMissile(-100, 100, PI/4);
        /*newRelativeHypersonicMissile(200, 0);
        newRelativeHypersonicMissile(-200, 0, PI);
        newRelativeHypersonicMissile(0, 200);
        newRelativeHypersonicMissile(0, -200);
        newRelativeHypersonicMissile(100, 0, PI/2);
        newRelativeHypersonicMissile(0, 100);
        newRelativeHypersonicMissile(0, -100, PI);
        newRelativeHypersonicMissile(-100, 100, PI/4);*/
    }
};


class ChestObject : public GameObject {
    int lives = 2;
    
    char identify() {
        return 'C'; // lowercase c = castle, uppercase C = chest
    }

    void destroy(){
        lives --;
        if (lives <= 0){
            rm = true;
            if (killer -> owner != NULL){
                killer -> owner -> collect(50);
            }
        }
    }

    void update() {};

    Box box () {
        return { x - 15, y - 15, x + 15, y + 15 };
    }
};


void NukeObject::blowUP(){
    RadiationObject* intenseField = new RadiationObject;
    intenseField -> x = x;
    intenseField -> y = y;
    intenseField -> mBox = {
        -50, -50, 50, 50
    };
    intenseField -> halflife = 60; // VERY short-lived
    intenseField -> startingStrength = 0.7; // But VERY strong field
    RadiationObject* weakField = new RadiationObject;
    weakField -> x = x;
    weakField -> y = y;
    weakField -> mBox = {
        -300, -300, 300, 300
    };
    weakField -> halflife = 250; // Comparatively long-lived
    weakField -> startingStrength = 0.2; // Slightly below average
    addObject(weakField);
    addObject(intenseField);
}


std::vector <Client*> clients;
std::mutex clientListMutex;

long micros(){
    struct timeval time;
    if (gettimeofday(&time, NULL) != 0){
        perror("Gettimeofday");
    }
    return time.tv_sec * 1000000 + time.tv_usec;
}

void randomObject(){
    bool chest = rand() % 2;
    GameObject* g;
    if (chest){
        g = new ChestObject;
    }
    else {
        g = new WallObject;
    }
    g -> x = rand() % gameSize;
    g -> y = rand() % gameSize;
    g -> goalX = g -> x;
    g -> goalY = g -> y;
    bool isGood = true;
    for (GameObject* object : objects){
        if (object -> identify() == 'c'){
            if ((std::abs(g -> x - object -> x) < 400) && (std::abs(g -> y - object -> y) < 400)){
                isGood = false;
            }
        }
    }
    if (isGood){
        addObject(g);
    }
    else {
        delete g;
    }
}


long newObjectTTL = 300;

void reset(){
    terrainSeed = rand();
    for (Client* cli : clients){
        delete cli;
    }
    clients.clear();
    for (GameObject* obj : objects){ // Delete the objects last, because the client deletion routines use them. I think.
        delete obj;
    }
    objects.clear();
    counter = 1;
    stratChangeMode = false;
    playing = false;
    banners.clear();
    autonomousTimer = 0;
    teamMode = false;
    for (team* t : teams){
        delete t;
    }
    teams.clear();
}

void start() {
    terminal.printLn("\033[1mStarting game.\033[0m");
    for (int x = 0; x < livePlayerCount * 5; x ++){
        randomObject();
    }
    playing = true;
}

void tick(){
    if (isAutonomous){
        if (livePlayerCount >= autonomousMinPlayers){
            autonomousTimer ++;
            if (autonomousTimer == autonomousStartTimer){
                start();
            }
            else if (autonomousTimer < autonomousStartTimer){
                broadcast("!" + std::to_string(autonomousStartTimer - autonomousTimer));
            }
        }
    }
    if (!playing){
        return;
    }
    size_t winner = -1;
    for (Client* cli : clients){
        if (cli -> is_authorized){
            if (winner == -1){
                winner = cli -> myTeam;
            }
            else {
                if (winner != cli -> myTeam){
                    winner = -1;
                    break;
                }
            }
        }
    }
    if (winner != -1){
        for (Client* cli : clients){
            if (cli -> is_authorized && (cli -> myTeam == winner)) {
                cli -> sendText("W");
            }
            else {
                cli -> sendText("E" + std::to_string(winner));
            }
        }
        terminal.printLn("\033[32mThe game ended with a winner! Resetting.\033[0m");
        reset();
    }
    if (livePlayerCount == 1){
        long winningBanner = -1;
        for (Client* cli : clients){
            if (cli -> is_authorized){
                cli -> sendText("W"); // You won the game!
                winningBanner = cli -> myBanner;
            }
        }
        for (Client* cli : clients){
            if (cli -> myBanner != winningBanner){
                cli -> sendText("E" + std::to_string(winningBanner)); // The game has ended, and the person identified by that banner one.
            }
        }
        terminal.printLn("\033[32mThe game ended with a winner! Resetting.\033[0m");
        reset();
    }
    if (livePlayerCount == 0){
        for (Client* cli : clients){
            cli -> sendText("T"); // The game was a tie.
        }
        terminal.printLn("\033[33mThe game ended with a tie. Resetting.\033[0m");
        reset();
    }
    counter --;
    if (counter == 0){
        stratChangeMode = !stratChangeMode;
        if (stratChangeMode){
            counter = FPS * 40; // 30 seconds in strat change mode
        }
        else{
            counter = FPS * 20; // For every 20 seconds of play. oooh this gon' be funnnnn
        }
    }
    if (!stratChangeMode){
        newObjectTTL --;
        if (newObjectTTL < 0){
            newObjectTTL = rand() % 200 + 50;
            randomObject();
        }
        for (size_t i = 0; i < objects.size(); i ++){
            GameObject* obj = objects[i];
            if (obj -> rm){
                clientListMutex.lock();
                delete obj;
                objects.erase(objects.begin() + i);
                clientListMutex.unlock();
            }
            else{
                obj -> update();
            }
        }
        for (size_t x = 0; x < objects.size() - 1; x ++){ // only go to the second-to-last, instead of the last, because the nested loop goes one above
            for (size_t y = x + 1; y < objects.size(); y ++){ // prevent double-collisions by starting this at one more than the end of the set of known complete collisions
                bool collided = false;
                if (objects[x] -> box().check(objects[y] -> box())){
                    char xWord = objects[x] -> identify();
                    char yWord = objects[y] -> identify();
                    if (xWord == 'c' || xWord == 'F'){
                        // If the collision root object is a castle or fort
                        if ((yWord == 'b') || (yWord == 'h') || (yWord == 'n') || (yWord == 'r')) {
                            collided = true;
                        }
                    }
                    else if (xWord == 'R'){
                        if ((yWord == 'b') || (yWord == 'h') || (yWord == 'n') || (yWord == 'r') || (yWord == 'C') || (yWord == 'w')) {
                            collided = true;
                        }
                    }
                    else if (xWord == 'b'){
                        // If the collision root object is a bullet
                        // bullets collide with everything.
                        collided = true;
                    }
                    else if (xWord == 'w' || xWord == 'C'){
                        if (yWord != 'c' && yWord != 'F'){
                            collided = true;
                        }
                    }
                    else if ((xWord == 'h') || (xWord == 'r') || (xWord == 'n')){
                        collided = true; // They hit *everything*
                    }
                    else { // If it's anything else, it's a fighter.
                        if (yWord != 'c' && yWord != 'R'){
                            collided = true;
                        }
                    }
                }
                if (collided){
                    objects[y] -> health -= objects[x] -> damage;
                    objects[x] -> health -= objects[y] -> damage;
                    if (objects[y] -> health <= 0){
                        objects[y] -> killer = objects[x];
                        objects[y] -> destroy();
                    }
                    if (objects[x] -> health <= 0){
                        objects[x] -> killer = objects[y];
                        objects[x] -> destroy();
                    }
                }
            }
        }
    }
    for (Client* client : clients){
        client -> tick(counter, stratChangeMode);
    }
}

bool isPasswordChange = false;


void* interactionThread(void* _){
    terminal.init(7);
    terminal.noncanonical();
    terminal.prompt = ": ";
    while (true){
        terminal[0] = (std::string)"\033[34m[ Living players] \033[0m\033[1m" + std::to_string(livePlayerCount) + "\033[0m";
        terminal[1] = (std::string)"\033[33m[ Total clients] \033[0m\033[1m" + std::to_string(clients.size()) + "\033[0m";
        terminal[2] = (std::string)"\033[32m[ Spectators] \033[0m\033[1m" + std::to_string(clients.size() - livePlayerCount) + "\033[0m";
        terminal[3] = (std::string)"\033[91m[ Total object count] \033[0m\033[1m" + std::to_string(objects.size()) + "\033[0m";
        terminal[4] = (std::string)"[ Current Game Size ] " + std::to_string(gameSize);
        terminal[5] = (std::string)"\033[36m[ Password ] \033[0m\033[1m" + code + "\033[0m";
        if (playing){
            terminal[6] = (std::string)"[ MODE ] " + (stratChangeMode ? "Strategy Change" : "Play");
        }
        else {
            terminal[6] = "[ MODE ] Waiting";
        }
        terminal.update();
        if (terminal.hasLine()){
            std::string command = terminal.getLine();
            if (command == "start"){
                start();
            }
            else if (command == "exit" || command == "quit"){
                break;
            }
            else if (command == "change password"){
                std::string password = terminal.input("New password: ");
                free(code);
                code = (char*)malloc(password.size() + 1);
                for (size_t i = 0; i < password.size(); i ++){
                    code[i] = password[i];
                }
                code[password.size()] = 0;
            }
            else if (command == "data"){
                std::map <char, long> breakdown;
                terminal.printLn("--- Client and Castle information ---");
                for (size_t i = 0; i < clients.size(); i ++) {
                    Client* client = clients[i];
                    if (client -> deMoi){
                        terminal.printLn("Client " + std::to_string(i) + " owns a castle at (" + std::to_string(client -> deMoi -> x) + ", " + std::to_string(client -> deMoi -> y) + "), with banner " + std::to_string(client -> myBanner) + " (sign " + banners[client -> myBanner] + ")." );
                    }
                    else {
                        terminal.printLn("Client " + std::to_string(i) + " (banner " + std::to_string(client -> myBanner) + ", sign " + banners[client -> myBanner] + ") does not own a castle.");
                    }
                }
                for (size_t i = 0; i < objects.size(); i ++) {
                    GameObject* obj = objects[i];
                    char sign = obj -> identify();
                    if (breakdown.contains(sign)){
                        breakdown[sign] ++;
                    }
                    else{
                        breakdown[sign] = 1;
                    }
                }
                terminal.printLn("--- Breakdown of objects by callsign (c = Castle, f = Fighter, t = Tiefighter, s = Sniper, b = Bullet, w = Wall, C = chest) ---");
                for (std::pair<const char, long>& object : breakdown){
                    //terminal.printLn("I hope mabel is ok :(");
                    terminal.printLn(object.first + (std::string)": " + std::to_string(object.second));
                }
            }
            else if (command == "broadcast"){
                broadcast("B" + terminal.input("Enter message to broadcast: "));
            }
            else if (command == "resize"){
                gameSize = std::stoi(terminal.input("Enter the new gameboard size: "));
                for (Client* cli : clients){
                    cli -> metadata(); // metadata changed - rebroadcast
                }
            }
            else if (command == "reset"){
                reset();
                terminal.printLn("### SERVER RESET ###");
            }
            else if (command == "clear unowned"){
                for (size_t i = 0; i < objects.size(); i ++){
                    if (objects[i] -> owner == NULL){
                        delete objects[i];
                        objects.erase(objects.begin() + i);
                        i --;
                    }
                }
            }
            else if (command == "skip stage"){
                counter = 1;
            }
            else if (command == "drop client"){
                size_t toKill = std::stoi(terminal.input("Client number to drop: "));
                for (size_t i = 0; i < objects.size(); i ++){
                    if (objects[i] -> owner == clients[toKill]){
                        delete objects[i];
                        objects.erase(objects.begin() + i);
                        i --;
                    }
                }
                clientListMutex.lock();
                delete clients[toKill];
                clients.erase(clients.begin() + toKill);
                clientListMutex.unlock();
            }
            else if (command == "autonomous"){
                autonomousMaxPlayers = std::stoi(terminal.input("Enter the maximum number of autonomous players: "));
                autonomousMinPlayers = std::stoi(terminal.input("Enter the minimum number of autonomous players: "));
                autonomousStartTimer = std::stoi(terminal.input("Enter the autonomous start timeout: "));
                isAutonomous = true;
            }
            else if (command == "teams"){
                terminal.printLn("=======\nBeginning Team Wizard\n=======");
                std::string it;
                while (true) {
                    it = terminal.input("Next team name ('done' to end): ");
                    std::string pass = terminal.input("Team password (empty for random): ");
                    char* password;
                    if (pass == ""){
                        password = randCode<6>();
                    }
                    else {
                        password = (char*)malloc(pass.size() + 1);
                        password[pass.size()] = 0;
                        for (size_t x = 0; x < pass.size(); x ++){
                            password[x] = pass[x];
                        }
                    }
                    if (it == "done"){
                        break;
                    }
                    else {
                        addBanner(it);
                        team* t = new team {
                            banners.size() - 1,
                            password
                        };
                        teams.push_back(t);
                        terminal.printLn("New team " + banners[t -> banner] + " created with password " + t -> password);
                    }
                }
                teamMode = true;
            }
            else {
                terminal.printLn("\033[31mUnrecognized command!\033[0m");
            }
        }
        usleep(50000); // 20 hertz update rate - shouldn't murder my computer
    }
    terminal.canonical();
    exit(1);
}

void* mainthread(void* _){
    double next_frame_us = micros();
    while (true){
        if (micros() > next_frame_us){
            next_frame_us += ALLOWED_MICROS_PER_FRAME;
            tick();
        }
    }
}


void addObject(GameObject* obj){
    objects.push_back(obj);
    obj -> id = GameObject::topId;
    GameObject::topId ++;
    for (Client* cli : clients){
        cli -> sendObject(obj);
    }
    obj -> greet();
}


void broadcast(std::string broadcast) {
    for (Client* cli : clients){
        cli -> sendText(broadcast);
    }
}


size_t getTeamOf(size_t banner){
    for (Client* cli : clients){
        if (banner == cli -> myBanner){
            return cli -> myTeam;
        }
    }
    return -1;
}


void GameObject::shoot(float angle, float speed, float dist, int duration){
    BulletObject* bullet = new BulletObject;
    bullet -> xv = cos(angle) * speed;
    bullet -> yv = sin(angle) * speed;
    bullet -> x = x + cos(angle) * dist;
    bullet -> y = y + sin(angle) * dist;
    bullet -> owner = owner;
    if (duration != -1){
        bullet -> TTL = duration;
    }
    addObject(bullet);
}


unsigned long GameObject::topId = 0;


int main(int argc, char** argv){
    int port = 3000;
    if (argc > 1){
        port = std::stoi(argv[1]);
    }
    FILE* random = fopen("/dev/urandom", "r");
    srand(fgetc(random));
    terrainSeed = rand();
    fclose(random);
    code = randCode<16>();
    std::cout << STARTING_SCREEN;
    /*std::cout << "┌───────────────────────────┐" << std::endl;
    std::cout << "│       \033[91;1mStrategy Game\033[0m       │" << std::endl;
    std::cout << "│      By Tyler Clarke      │" << std::endl;
    std::cout << "└───────────────────────────┘" << std::endl;
    std::cout << "Type 'start' and press enter to begin the game." << std::endl;
    std::cout << "\033[32mSharing code: " << code << "\033[0m" << std::endl;*/
    pthread_t thread, interaction;
    pthread_create(&thread, NULL, mainthread, NULL);
    pthread_detach(thread);
    pthread_create(&interaction, NULL, interactionThread, NULL);
    pthread_detach(interaction);
    crow::SimpleApp webserver;
    webserver.loglevel(crow::LogLevel::Warning);
    CROW_ROUTE(webserver, "/")([](const crow::request& req, crow::response& res){
        res.set_static_file_info("index.html");
        res.end();
    });
    CROW_ROUTE(webserver, "/images/<path>")([](const crow::request& req, crow::response& res, std::string path){
        res.set_static_file_info("images/" + path);
        //terminal.printLn("images/" + path);
        res.end();
    });
    CROW_ROUTE(webserver, "/game").websocket().onopen([](crow::websocket::connection& conn){
        clientListMutex.lock();
        clients.push_back(new Client {conn});
        clientListMutex.unlock();
    }).onmessage([](crow::websocket::connection& conn, std::string message, bool is_binary){
        for (Client* cli : clients){
            if (&cli -> conn == &conn){
                cli -> process(message);
            }
        }
    }).onclose([](crow::websocket::connection& conn, std::string reason){
        clientListMutex.lock();
        for (size_t x = 0; x < clients.size(); x ++){
            if (&clients[x] -> conn == &conn){
                delete clients[x];
                clients.erase(clients.begin() + x);
                x --;
                terminal.printLn("Dropped client.");
            }
        }
        clientListMutex.unlock();
    });
    webserver.port(port).multithreaded().run();
    return 0;
}
