// Gamepiece code
use crate::physics::*;
use crate::ServerToClient;
use crate::Server;
use std::f32::consts::PI;
use crate::vector::Vector2;
pub mod fighters;
pub mod misc;
pub mod npc;
pub mod nexus;
use crate::functions::coterminal;


#[derive(Clone)]
pub struct ShooterProperties {
    shoot         : bool,
    pub counter   : u32,
    angles        : Vec<f32>,
    range         : i32,
    pub suppress  : bool, // It can't shoot if this is on, but it can count down.
    bullet_type   : BulletType
}


#[derive(Clone)]
pub struct HealthProperties {
    pub max_health        : f32,
    pub health            : f32,
    passive_heal          : f32,
    prevent_friendly_fire : bool
}


#[derive(Clone)]
pub enum TargetingFilter {
    Any,
    Fighters,
    Castles,
    RealTimeFighter,
    Farmer
}


#[derive(PartialEq, Clone)]
pub enum TargetingMode {
    None,
    Nearest,
    Id (u32)
}


#[derive(Clone, Copy)]
pub enum BulletType {
    Bullet,
    AntiRTF,
    Laser (f32, f32) // laser intensity, laser range
}

#[derive(Clone, Copy)]
pub struct RepeaterProperties {
    repeats : u16,
    max_repeats : u16,
    repeat_cd : u32
}


#[derive(Debug)]
pub enum ReqZone { // Placing zone.
    NoZone, // can place anywhere
    WithinCastleOrFort, // some defensive objects can *also* be placed around forts.
    WithinCastle, // most common one: can only place inside your sphere of influence
    AwayFromThings, // castles and forts are this: cannot be placed near things
    Both
}


#[derive(Clone)]
pub struct Targeting {
    mode      : TargetingMode,
    filter    : TargetingFilter,
    range     : (f32, f32),
    vector_to : Option<Vector2>
}


#[derive(Clone)]
pub struct CarrierProperties {
    pub space_remaining : u32, // amount of remaining carrier space
    pub carrying : Vec<u32>, // list of things it is carrying
    pub does_accept : Vec<char>, // list of types it'll carry
    pub can_update : bool, // if it can update safely while being carried
    pub is_carried : bool, // if it's being carried at the moment. objects being carried cannot shoot and don't do any collision damage.
    pub berth      : usize, // if it's being carried, this is the berth it's in.
    pub carrier    : u32 // who is carrying us
}


impl CarrierProperties {
    pub fn will_carry(&self, thing : char) -> bool {
        self.does_accept.contains(&thing) && self.space_remaining > 0
    }
}


#[derive(Clone)]
pub struct ExposedProperties { // everything a GamePieceBase wants to expose to GamePieces
    pub collision_info     : CollisionInfo,
    pub physics            : PhysicsObject,
    pub shooter_properties : ShooterProperties,
    pub health_properties  : HealthProperties,
    pub targeting          : Targeting,
    pub carrier_properties : CarrierProperties,
    pub exploder           : Vec<ExplosionMode>,
    pub id                 : u32,
    pub goal_x             : f32,
    pub value              : char,
    pub goal_y             : f32,
    pub goal_a             : f32,
    pub ttl                : i32, // ttl of < 0 means ttl does nothing. ttl of 0 means die. ttl of anything higher means subtract one every update.
    pub repeater           : RepeaterProperties,
    pub banner             : usize
}


#[derive(Clone)]
pub enum ExplosionMode {
    None,
    Radiation (f32, f32, f32)
}


pub trait GamePiece {
    fn construct<'a>(&'a self, _properties : &mut ExposedProperties) {
        
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::WithinCastle
    }

    fn identify(&self) -> char;

    fn obtain_physics(&self) -> PhysicsObject;

    fn on_die(&mut self, _banner : usize, _servah : &mut Server) {

    }
    
    fn is_editable(&self) -> bool {
        false
    }

    fn get_does_collide(&self, _id : char) -> bool {
        true
    }

    fn update(&mut self, _properties : &mut ExposedProperties, _servah : &mut Server) {
        
    }

    fn cost(&self) -> i32 {
        0
    }

    fn capture(&self) -> u32 {
        std::cmp::min(((self.cost() * 3) / 2) as u32, 75) // The most you can score on any capture is, by default, 75
    }

    fn on_upgrade(&mut self, _properties : &mut ExposedProperties, _upgrade : &String) {

    }

    fn on_carry(&mut self, _properties : &mut ExposedProperties, _thing : &mut ExposedProperties, _server : &mut Server) { // when a new object becomes carried by this

    }

    fn carry_iter(&mut self, _properties : &mut ExposedProperties, _thing : &mut ExposedProperties) -> bool { // called to iterate over every carried object every update
        false
    }

    fn drop_carry(&mut self, _properties : &mut ExposedProperties, _thing : &mut ExposedProperties) { // called to iterate over every carried object every update
        
    }

    fn does_grant_a2a(&self) -> bool {
        false
    }

    fn do_stream_health(&self) -> bool {
        false
    }

    fn on_subscribed_death(&mut self, _me : &mut ExposedProperties, _them : &mut GamePieceBase, _servah : &mut Server) {

    }
}


#[derive(Copy, Clone)]
pub struct CollisionInfo {
    pub damage  : f32, // Damage done constantly to any objects colliding with this object
    pub worthit : bool
}

pub struct GamePieceBase {
    banner                 : usize,
    pub exposed_properties : ExposedProperties,
    pub piece              : Box<dyn GamePiece + Send + Sync>, // public because the server has to touch it on occasion
    pub shoot_timer        : u32,
    broadcasts             : Vec<ServerToClient>,
    forts                  : Vec<u32>,
    pub upgrades           : Vec<String>,
    pub zones              : Vec<usize>,
    pub death_subscriptions: Vec<u32>
}

impl GamePieceBase {
    pub fn new(piece : Box<dyn GamePiece + Send + Sync>, x : f32, y : f32, a : f32) -> Self {
        let mut physics = piece.obtain_physics(); // Get configured physics and shape
        physics.set_cx(x); // Set the position, because the shape don't get to decide that (yet)
        physics.set_cy(y);
        physics.set_angle(a);
        let mut thing = Self {
            banner : 0,
            shoot_timer : 20,
            zones : vec![],
            exposed_properties : ExposedProperties {
                health_properties : HealthProperties {
                    max_health : 2.0,
                    health : 1.0,
                    passive_heal : 0.0,
                    prevent_friendly_fire : false
                },
                banner: 0,
                shooter_properties : ShooterProperties {
                    shoot : false,
                    counter : 0,
                    angles : vec![0.0],
                    range : 30,
                    suppress : false,
                    bullet_type : BulletType::Bullet
                },
                value : piece.identify(),
                collision_info : CollisionInfo {
                    damage : 1.0,
                    worthit: true
                },
                targeting : Targeting {
                    mode : TargetingMode::None,
                    filter : TargetingFilter::Any,
                    range : (0.0, 0.0),
                    vector_to : None
                },
                goal_x : x,
                goal_y : y,
                goal_a : physics.angle(),
                physics,
                ttl : -1,
                exploder : vec![],
                id : 0,
                carrier_properties : CarrierProperties {
                    space_remaining : 0,
                    carrying : vec![],
                    does_accept : vec![],
                    is_carried : false,
                    can_update : false,
                    berth : 0,
                    carrier : 0
                },
                repeater : RepeaterProperties {
                    repeats     : 0,
                    max_repeats : 0,
                    repeat_cd   : 5 // default, you don't usually have to touch this
                }
            },
            broadcasts : vec![],
            forts : vec![],
            piece,
            upgrades : vec![],
            death_subscriptions : vec![]
        };
        thing.piece.construct(&mut thing.exposed_properties);
        thing.exposed_properties.health_properties.health = thing.exposed_properties.health_properties.max_health;
        thing.exposed_properties.repeater.repeats = thing.exposed_properties.repeater.max_repeats;
        thing
    }

    pub fn does_give_score(&self) -> bool {
        self.exposed_properties.collision_info.worthit
    }

    pub fn on_subscribed_death(&mut self, other : &mut GamePieceBase, server : &mut Server) {
        self.piece.on_subscribed_death(&mut self.exposed_properties, other, server);
    }

    pub fn death_subscribe(&mut self, other : u32) {
        self.death_subscriptions.push(other);
    }

    pub fn identify(&self) -> char {
        self.exposed_properties.value
    }

    pub fn is_editable(&self) -> bool {
        self.piece.is_editable()
    }

    pub fn murder(&mut self) {
        self.exposed_properties.health_properties.health = 0.0;
    }

    pub fn does_grant_a2a(&self) -> bool {
        self.piece.does_grant_a2a()
    }

    pub fn update_carried(&mut self, server : &mut Server) {
        for i in 0..self.exposed_properties.carrier_properties.carrying.len() {
            let obj = server.obj_lookup(self.exposed_properties.carrier_properties.carrying[i]);
            match obj {
                Some(obj) => {
                    if self.piece.carry_iter(&mut self.exposed_properties, &mut server.objects[obj].exposed_properties) { // drop the carried object
                        self.piece.drop_carry(&mut self.exposed_properties, &mut server.objects[obj].exposed_properties);
                        server.send_to(ServerToClient::UnCarry (server.objects[obj].get_id()), server.objects[obj].get_banner());
                        self.exposed_properties.carrier_properties.space_remaining += 1;
                        server.objects[obj].exposed_properties.carrier_properties.is_carried = false;
                    }
                }
                None => {}
            }
        }
        let mut i : usize = 0;
        while i < self.exposed_properties.carrier_properties.carrying.len() { // remove everything from the list AFTER they've been properly released, so reordering doesn't cause problems above
            let obj = server.obj_lookup(self.exposed_properties.carrier_properties.carrying[i]);
            match obj {
                Some(obj) => {    // TODO: optimize, we should only need one lookup per object for this instead of 2.
                    if !server.objects[obj].exposed_properties.carrier_properties.is_carried { // if it's been marked not-carried, so we still have an uncarried object in our carry list - problematic!
                        self.exposed_properties.carrier_properties.carrying.remove(i);
                        continue; // don't let it increment i
                    }
                }
                None => {}
            }
            i += 1;
        }
    }

    pub fn target(&mut self, server : &mut Server) {
        let mut best : Option<usize> = None;
        let mut best_value : f32 = 0.0; // If best is None, this value is ignored, so it can be anything.
        // The goal here is to compare the entire list of objects by some easily derived numerical component,
        // based on a set of options stored in targeting, and set the values in targeting based on that.
        // NOTE: the comparison is *always* <; if you want to compare > values multiply by negative 1.
        let mut carrier = None;
        if self.exposed_properties.carrier_properties.is_carried {
            carrier = server.obj_lookup(self.exposed_properties.carrier_properties.carrier);
        }
        for i in 0..server.objects.len() {
            let object = &server.objects[i];
            if object.get_id() == self.get_id() {
                continue;
            }
            if object.get_banner() != 0 && (object.get_banner() == self.get_banner() || (server.get_team_of_banner(object.get_banner()) == server.get_team_of_banner(self.get_banner())) && server.get_team_of_banner(self.get_banner()).is_some()) { // If you're under the same flag, skip.
                continue;
            }
            let mut viable = match self.exposed_properties.targeting.filter {
                TargetingFilter::Any => {
                    true
                },
                TargetingFilter::Fighters => {
                    match object.identify() {
                        'f' | 'h' | 'R' | 't' | 's' | '&' | 'C' => true,
                        _ => false
                    }
                },
                TargetingFilter::Castles => {
                    match object.identify() {
                        'R' | 'c' => true,
                        _ => false
                    }
                },
                TargetingFilter::RealTimeFighter => {
                    object.identify() == 'R'
                },
                TargetingFilter::Farmer => {
                    match object.identify() {
                        'C' | 'h' | 'b' => true,
                        _ => false
                    }
                }
            };
            match carrier {
                Some(carrier) => {
                    let mut bullet = object.exposed_properties.physics.vector_position() - self.exposed_properties.physics.vector_position(); // anticipate a bullet position. we won't target this if shooting at it would damage the carrier.
                    bullet.set_magnitude(50.0);
                    bullet += self.exposed_properties.physics.vector_position();
                    // TODO: check more bullet possibilities
                    if server.objects[carrier].exposed_properties.physics.shape.contains(bullet) {
                        viable = false; // this is not a viable match if firing on it would damage our carrier
                    }
                }
                None => {}
            }
            if viable {
                let val = match self.exposed_properties.targeting.mode {
                    TargetingMode::Nearest => {
                        let dist = (object.exposed_properties.physics.vector_position() - self.exposed_properties.physics.vector_position()).magnitude();
                        if (dist >= self.exposed_properties.targeting.range.0 && dist <= self.exposed_properties.targeting.range.1) || self.exposed_properties.targeting.range.1 == 0.0 {
                            Some(dist)
                        }
                        else {
                            None
                        }
                    },
                    TargetingMode::Id (id) => {
                        if object.exposed_properties.id == id {
                            Some(0.0) // the id is always the best possibility
                        }
                        else {
                            None
                        }
                    },
                    TargetingMode::None => None
                };
                if val.is_some() {
                    if val.unwrap() < best_value || !best.is_some() {
                        best_value = val.unwrap();
                        best = Some(i);
                    }
                }
            }
        }
        match best {
            Some(best) => {
                self.exposed_properties.targeting.vector_to = Some(server.objects[best].exposed_properties.physics.vector_position() - self.exposed_properties.physics.vector_position());
            }
            None => {
                self.exposed_properties.targeting.vector_to = None;
            }
        }
    }

    pub fn broadcast(&mut self, message : ServerToClient) {
        self.broadcasts.push(message);
    }

    pub async fn on_carry(&mut self, mut thing : GamePieceBase, server : &mut Server) {
        /*self.exposed_properties.carrier_properties.carrying.push(thing.get_id());
        self.exposed_properties.carrier_properties.space_remaining -= 1;
        thing.exposed_properties.carrier_properties.is_carried = true;
        thing.exposed_properties.physics.velocity = Vector2::empty();*/
        self.piece.on_carry(&mut self.exposed_properties, &mut thing.exposed_properties, server);
    }

    pub fn update(&mut self, server : &mut Server) {
        if self.exposed_properties.carrier_properties.is_carried {
            self.exposed_properties.health_properties.health = self.exposed_properties.health_properties.max_health;
        }
        if self.exposed_properties.carrier_properties.is_carried && !self.exposed_properties.carrier_properties.can_update {
            return; // quick short circuit: can't update if it's being carried, carriers freeze all activity so it's nice and ready for when it comes back out
        }
        if self.piece.do_stream_health() {
            server.stream_health(self.exposed_properties.id, self.exposed_properties.health_properties.health / self.exposed_properties.health_properties.max_health);
        }
        let mut i : usize = 0;
        while i < self.forts.len() {
            match server.obj_lookup(self.forts[i]) {
                Some(_) => {},
                None => { // the fort object is dead, throw away the id
                    self.forts.remove(i);
                    continue; // Don't allow i to increment
                }
            }
            i += 1;
        }
        if self.exposed_properties.health_properties.health <= 0.0 && self.forts.len() > 0 {
            let fortid = self.forts.remove(0); // pop out the oldest fort in the list
            let fort = server.obj_lookup(fortid).unwrap(); // the previous loop removes forts that don't exist, so at this point in the code it must be safe to unwrap
            server.objects[fort].exposed_properties.health_properties.health = -1.0; // kill the fort
            self.exposed_properties.health_properties.health = self.exposed_properties.health_properties.max_health; // Restore to maximum health.
            self.exposed_properties.physics.set_cx(server.objects[fort].exposed_properties.physics.cx());
            self.exposed_properties.physics.set_cy(server.objects[fort].exposed_properties.physics.cy());
            return; // Don't die yet! You have a fort!
        }
        if self.exposed_properties.targeting.mode != TargetingMode::None {
            self.target(server);
        }
        self.exposed_properties.physics.update();
        self.piece.update(&mut self.exposed_properties, server);
        self.update_carried(server);
        if self.exposed_properties.physics.portals {
            self.exposed_properties.physics.set_cx(coterminal(self.exposed_properties.physics.cx(), server.gamesize as f32));
            self.exposed_properties.physics.set_cy(coterminal(self.exposed_properties.physics.cy(), server.gamesize as f32));
        }
        if self.exposed_properties.health_properties.health < self.exposed_properties.health_properties.max_health {
            self.exposed_properties.health_properties.health += self.exposed_properties.health_properties.passive_heal;
        }
        if self.exposed_properties.health_properties.health > self.exposed_properties.health_properties.max_health {
            self.exposed_properties.health_properties.health = self.exposed_properties.health_properties.max_health;
        }
        if self.exposed_properties.shooter_properties.shoot {
            if self.exposed_properties.shooter_properties.suppress {
                if self.shoot_timer > 0 {
                    self.shoot_timer -= 1;
                }
            }
            else {
                if self.shoot_timer == 0 {
                    self.shoot_timer = self.exposed_properties.shooter_properties.counter;
                    self.shawty(self.exposed_properties.shooter_properties.range, server);
                    if self.exposed_properties.repeater.repeats > 0 {
                        self.exposed_properties.repeater.repeats -= 1;
                        self.shoot_timer = self.exposed_properties.repeater.repeat_cd;
                    }
                    else {
                        self.exposed_properties.repeater.repeats = self.exposed_properties.repeater.max_repeats;
                    }
                }
                else {
                    self.shoot_timer -= 1;
                }
            }
        }
        if self.exposed_properties.ttl > 0 {
            self.exposed_properties.ttl -= 1;
        }
        else if self.exposed_properties.ttl == 0 {
            self.exposed_properties.health_properties.health = 0.0;
        }
        while self.broadcasts.len() > 0 {
            server.broadcast(self.broadcasts.remove(0));
        }
        if self.exposed_properties.physics.speed_cap != 0.0 {
            if self.exposed_properties.physics.velocity.magnitude() > self.exposed_properties.physics.speed_cap {
                self.exposed_properties.physics.velocity.set_magnitude(self.exposed_properties.physics.speed_cap);
            }
        }
    }

    pub fn shawty(&mut self, range : i32, server : &mut Server) {
        for angle in &self.exposed_properties.shooter_properties.angles {
            if let BulletType::Laser (intensity, range) = self.exposed_properties.shooter_properties.bullet_type {
                server.fire_laser(self.exposed_properties.physics.extend_point(50.0, *angle) + self.exposed_properties.physics.velocity, *angle + self.exposed_properties.physics.angle(), intensity, range, self.identify(), Some(self.banner));
            }
            else {
                let bullet_id = server.shoot(self.exposed_properties.shooter_properties.bullet_type, self.exposed_properties.physics.extend_point(50.0, *angle), Vector2::new_from_manda(20.0, self.exposed_properties.physics.angle() + *angle) + self.exposed_properties.physics.velocity, range, None);
                let bullet = server.obj_lookup(bullet_id).unwrap(); // Unwrap is safe here because the object is guaranteed to exist at this point.
                server.objects[bullet].set_banner(self.banner); // Set the banner.
            }
        }
    }

    pub fn health(&self) -> f32 {
        self.exposed_properties.health_properties.health
    }

    pub fn damage(&mut self, harm : f32) {
        self.exposed_properties.health_properties.health -= harm;
    }
    
    pub fn dead(&self) -> bool {
        self.exposed_properties.health_properties.health <= 0.0 && self.forts.len() == 0
    }

    pub fn die(&mut self, server : &mut Server) {
        self.piece.on_die(self.banner, server);
        for explosion in &self.exposed_properties.exploder {
            match explosion {
                ExplosionMode::Radiation(size, halflife, strength) => {
                    server.place_radiation(self.exposed_properties.physics.cx(), self.exposed_properties.physics.cy(), *size, *halflife, *strength, self.exposed_properties.physics.angle(), None);
                },
                _ => {

                }
            }
        }
        for carried in &self.exposed_properties.carrier_properties.carrying {
            let obj = server.obj_lookup(*carried).unwrap(); // if it's being carried, it's guaranteed to not be deleted, so unwrapping is safe.
            // MAY BE PROBLEMATIC!
            server.objects[obj].exposed_properties.carrier_properties.is_carried = false;
            server.send_to(ServerToClient::UnCarry (*carried), server.objects[obj].get_banner());
            server.objects[obj].exposed_properties.goal_x = server.objects[obj].exposed_properties.physics.cx();
            server.objects[obj].exposed_properties.goal_y = server.objects[obj].exposed_properties.physics.cy();
            server.objects[obj].exposed_properties.goal_a = server.objects[obj].exposed_properties.physics.angle();
        }
    }

    pub fn get_does_collide(&self, id : char) -> bool {
        self.piece.get_does_collide(id)
    }

    pub fn get_physics_object(&mut self) -> &mut PhysicsObject {
        &mut self.exposed_properties.physics
    }

    pub fn get_collision_info(&self) -> CollisionInfo {
        self.exposed_properties.collision_info
    }

    pub fn get_new_message(&self) -> ServerToClient {
        ServerToClient::New (
            self.get_id(),
            self.identify() as u8,
            self.exposed_properties.physics.cx(),
            self.exposed_properties.physics.cy(),
            self.exposed_properties.physics.angle(),
            self.is_editable(),
            self.get_banner() as u32,
            self.exposed_properties.physics.width(),
            self.exposed_properties.physics.height()
        )
    }

    pub fn get_banner(&self) -> usize {
        self.banner
    }

    pub fn set_banner(&mut self, new : usize) {
        self.banner = new;
        self.exposed_properties.banner = new;
    }

    pub fn set_id(&mut self, id : u32) {
        self.exposed_properties.id = id;
    }

    pub fn get_id(&self) -> u32 {
        self.exposed_properties.id
    }

    pub fn cost(&self) -> i32 {
        self.piece.cost()
    }

    pub fn get_health_perc(&self) -> f32 {
        self.exposed_properties.health_properties.health/self.exposed_properties.health_properties.max_health
    }

    pub fn capture(&self) -> u32 {
        self.piece.capture()
    }

    pub fn add_fort(&mut self, fortid : u32) {
        self.forts.push(fortid);
    }

    pub fn get_max_health(&self) -> f32 {
        self.exposed_properties.health_properties.max_health
    }

    pub fn upgrade(&mut self, up : String) {
        self.piece.on_upgrade(&mut self.exposed_properties, &up);
        self.upgrades.push(up);
    }
}


pub struct Castle {
    is_rtf : bool
}
pub struct Fort {}


impl Castle {
    pub fn new(is_rtf : bool) -> Self {
        Self {
            is_rtf
        }
    }
}

impl Fort {
    pub fn new() -> Self {
        Self {}
    }
}


impl GamePiece for Castle {
    fn obtain_physics(&self) -> PhysicsObject {
        if self.is_rtf {
            PhysicsObject::new(0.0, 0.0, 18.0, 80.0, 0.0)
        }
        else{
            PhysicsObject::new(0.0, 0.0, 50.0, 50.0, 0.0)
        }
    }

    fn on_die(&mut self, banner : usize, server : &mut Server) {
        server.player_died(banner, self.is_rtf);
        println!("bluh");
    }

    fn do_stream_health(&self) -> bool {
        true
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::AwayFromThings
    }

    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 3.0;
        if self.is_rtf {
            thing.shooter_properties.counter = 15;
            thing.shooter_properties.shoot = true;
            thing.shooter_properties.angles[0] = -PI/2.0;
            thing.shooter_properties.suppress = true;
            thing.physics.speed_cap = 20.0;
            thing.physics.portals = true;
            thing.health_properties.passive_heal = 0.002;
        }
    }

    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        if !self.is_rtf {
            properties.physics.velocity = Vector2::empty();
        }
    }

    fn identify(&self) -> char {
        if self.is_rtf { 'R' } else { 'c' }
    }

    fn get_does_collide(&self, id : char) -> bool {
        if self.is_rtf {
            id != 'c' // The only thing RTFs don't collide with is castles. After all, they *are* a type of fighter.
        }
        else {
            id == 'b' || id == 'r' || id == 'h' // All castles collide with is bullets and radiation.
        }
    }

    fn capture(&self) -> u32 {
        150
    }

    fn on_upgrade(&mut self, properties : &mut ExposedProperties, upgrade : &String) {
        /*
Upgrade tiers
**Gun**:
1. faster gun, but not nearly as fast as the current faster gun
2. twice repeater gun
3. much better range
4. three-prong shooter
**Cloaking**:
1. sniper
**Drive**:
1. faster RTF
2. even faster RTF
3. better turns (more controlled, but also faster)
**Health**:
1. Higher regen
2. Higher maxhealth
3. Much higher regen
4. Much higher maxhealth
        */
        match upgrade.as_str() {
            "b" => { // shot counter speed
                properties.shooter_properties.counter = 12;
            },
            "b2" => {
                properties.repeater.max_repeats = 1;
                properties.repeater.repeat_cd = 1;
            },
            "b3" => {
                properties.shooter_properties.range = 80;
            }
            "b4" => { // high-intensity laser
                properties.shooter_properties.bullet_type = BulletType::Laser (3.0, 5000.0);
                properties.shooter_properties.counter = 10;
                properties.repeater.max_repeats = 0;
            }
            "f" => { // fast
                properties.physics.speed_cap = 30.0;
            },
            "f2" => { // faster
                properties.physics.speed_cap = 50.0;
            },
            "h" => { // heal
                properties.health_properties.passive_heal = 0.005;
            },
            "h2" => {
                properties.health_properties.max_health = 5.0;
            },
            "h3" => {
                properties.health_properties.passive_heal = 0.01;
            },
            "h4" => {
                properties.health_properties.max_health = 8.0;
            },
            &_ => {
                
            }
        }
    }
}

impl GamePiece for Fort {
    fn identify(&self) -> char {
        'F'
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::Both
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 10.0, 10.0, 0.0)
    }

    fn cost(&self) -> i32 {
        120
    }
}
