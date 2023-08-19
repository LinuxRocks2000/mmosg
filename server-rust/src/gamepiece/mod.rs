// Gamepiece code
use crate::physics::*;
use crate::ProtocolMessage;
use crate::Server;
use std::f32::consts::PI;
use crate::vector::Vector2;
pub mod fighters;
pub mod misc;
pub mod npc;
use crate::functions::coterminal;


#[derive(Clone)]
pub struct ShooterProperties {
    shoot         : bool,
    counter       : u32,
    angles        : Vec<f32>,
    range         : i32,
    pub suppress  : bool, // It can't shoot if this is on, but it can count down.
    bullet_type   : BulletType
}


#[derive(Clone)]
pub struct HealthProperties {
    max_health   : f32,
    health       : f32,
    passive_heal : f32
}


#[derive(Clone)]
pub enum TargetingFilter {
    Any,
    Fighters,
    Castles,
    RealTimeFighter
}


#[derive(PartialEq, Clone)]
pub enum TargetingMode {
    None,
    Nearest
}


#[derive(Clone, Copy)]
pub enum BulletType {
    Bullet,
    AntiRTF
}


#[derive(Debug)]
pub enum ReqZone { // Placing zone.
    NoZone, // can place anywhere
    WithinCastleOrFort, // most common one: can only place inside your sphere of influence
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
    pub carrying : Vec<Arc<Mutex<GamePieceBase>>>, // list of things it is carrying
    pub does_accept : Vec<char>, // list of types it'll carry
    pub is_carried : bool // if it's being carried at the moment. objects being carried cannot shoot and don't do any collision damage.
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
    pub goal_y             : f32,
    pub goal_a             : f32,
    pub ttl                : i32, // ttl of < 0 means ttl does nothing. ttl of 0 means die. ttl of anything higher means subtract one every update.
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
        ReqZone::WithinCastleOrFort
    }

    fn identify(&self) -> char;

    fn obtain_physics(&self) -> PhysicsObject;

    fn on_die(&mut self) {

    }
    
    fn is_editable(&self) -> bool {
        false
    }

    fn get_does_collide(&self, _id : char) -> bool {
        true
    }

    fn update(&mut self, _properties : &mut ExposedProperties, _servah : &mut Server) {
        
    }

    fn cost(&self) -> u32 {
        0
    }

    fn capture(&self) -> u32 {
        std::cmp::min(((self.cost() * 3) / 2) as u32, 75) // The most you can score on any capture is, by default, 75
    }

    fn on_upgrade(&mut self, _properties : &mut ExposedProperties, _upgrade : Arc<String>) {

    }

    fn on_carry(&mut self, _properties : &mut ExposedProperties, _thing : &mut ExposedProperties) { // when a new object becomes carried by this

    }

    fn carry_iter(&mut self, _properties : &mut ExposedProperties, _thing : &mut ExposedProperties, _index : usize) -> bool { // called to iterate over every carried object every update
        false
    }

    fn drop_carry(&mut self, _properties : &mut ExposedProperties, _thing : &mut ExposedProperties, _index : usize) { // called to iterate over every carried object every update
        
    }
}


#[derive(Copy, Clone)]
pub struct CollisionInfo {
    pub damage : f32, // Damage done constantly to any objects colliding with this object
}

pub struct GamePieceBase {
    banner                 : usize,
    pub exposed_properties : ExposedProperties,
    value                  : char,
    piece                  : Box<dyn GamePiece + Send + Sync>,
    pub shoot_timer        : u32,
    broadcasts             : Vec<ProtocolMessage>,
    forts                  : Vec<Arc<Mutex<GamePieceBase>>>,
    upgrades               : Vec<Arc<String>>
}

use tokio::sync::Mutex;
use std::sync::Arc;

impl GamePieceBase {
    pub fn new(piece : Box<dyn GamePiece + Send + Sync>, x : f32, y : f32, a : f32) -> Self {
        let mut physics = piece.obtain_physics(); // Get configured physics and shape
        physics.set_cx(x); // Set the position, because the shape don't get to decide that (yet)
        physics.set_cy(y);
        physics.set_angle(a);
        let mut thing = Self {
            banner : 0,
            value : piece.identify(),
            piece,
            shoot_timer : 20,
            exposed_properties : ExposedProperties {
                health_properties : HealthProperties {
                    max_health : 2.0,
                    health : 1.0,
                    passive_heal : 0.0
                },
                shooter_properties : ShooterProperties {
                    shoot : false,
                    counter : 0,
                    angles : vec![0.0],
                    range : 30,
                    suppress : false,
                    bullet_type : BulletType::Bullet
                },
                collision_info : CollisionInfo {
                    damage : 1.0
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
                    is_carried : false
                }
            },
            broadcasts : vec![],
            forts : vec![],
            upgrades : vec![]
        };
        thing.piece.construct(&mut thing.exposed_properties);
        thing.exposed_properties.health_properties.health = thing.exposed_properties.health_properties.max_health;
        thing
    }

    pub fn identify(&self) -> char {
        self.value
    }

    pub fn is_editable(&self) -> bool {
        self.piece.is_editable()
    }

    pub fn murder(&mut self) {
        self.exposed_properties.health_properties.health = 0.0;
    }

    pub async fn target(&mut self, server : &mut Server) {
        let mut best : Option<Arc<Mutex<GamePieceBase>>> = None;
        let mut best_value : f32 = 0.0; // If best is None, this value is ignored, so it can be anything.
        // The goal here is to compare the entire list of objects by some easily derived numerical component,
        // based on a set of options stored in targeting, and set the values in targeting based on that.
        // NOTE: the comparison is *always* <; if you want to compare > values multiply by negative 1.
        for locked in &server.objects {
            match locked.try_lock() {
                Ok(object) => {
                    if object.get_banner() == self.get_banner() { // If you're under the same flag, skip.
                        continue;
                    }
                    let viable = match self.exposed_properties.targeting.filter {
                        TargetingFilter::Any => {
                            true
                        },
                        TargetingFilter::Fighters => {
                            match object.identify() {
                                'f' | 'h' | 'R' | 't' | 's' => true,
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
                        }
                    };
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
                            TargetingMode::None => None
                        };
                        if val.is_some() {
                            if val.unwrap() < best_value || !best.is_some() {
                                best_value = val.unwrap();
                                best = Some(locked.clone());
                            }
                        }
                    }
                },
                Err(_) => {
                    // It's us, so don't worry about it: do nothing.
                }
            }
        }
        if best.is_some() {
            self.exposed_properties.targeting.vector_to = Some(best.unwrap().lock().await.exposed_properties.physics.vector_position() - self.exposed_properties.physics.vector_position());
        }
        else {
            self.exposed_properties.targeting.vector_to = None;
        }
    }

    pub fn broadcast(&mut self, message : ProtocolMessage) {
        self.broadcasts.push(message);
    }

    pub async fn on_carry(&mut self, thing : Arc<Mutex<GamePieceBase>>) {
        self.exposed_properties.carrier_properties.carrying.push(thing.clone());
        self.exposed_properties.carrier_properties.space_remaining -= 1;
        let mut lock = thing.lock().await;
        lock.exposed_properties.carrier_properties.is_carried = true;
        lock.exposed_properties.physics.velocity = Vector2::empty();
        self.piece.on_carry(&mut self.exposed_properties, &mut lock.exposed_properties);
    }

    pub async fn update(&mut self, server : &mut Server) {
        if self.exposed_properties.carrier_properties.is_carried {
            return; // quick short circuit: can't update if it's being carried, carriers freeze all activity so it's nice and ready for when it comes back out
        }
        let mut i : usize = 0;
        while i < self.forts.len() {
            if self.forts[i].lock().await.dead() {
                self.forts.remove(i);
                continue; // Don't allow i to increment
            }
            i += 1;
        }
        if self.exposed_properties.health_properties.health <= 0.0 && self.forts.len() > 0 { // DEADLOCK CONDITION: the fort is circularly linked. Unlikely.
            let fortex = self.forts.remove(0); // pop out the oldest fort in the list
            let mut fort = fortex.lock().await;
            fort.exposed_properties.health_properties.health = -1.0; // kill the fort
            self.exposed_properties.health_properties.health = self.exposed_properties.health_properties.max_health; // Restore to maximum health.
            self.exposed_properties.physics.set_cx(fort.exposed_properties.physics.cx());
            self.exposed_properties.physics.set_cy(fort.exposed_properties.physics.cy());
            return; // Don't die yet! You have a fort!
        }
        if self.exposed_properties.targeting.mode != TargetingMode::None {
            self.target(server).await;
        }
        self.exposed_properties.physics.update();
        self.piece.update(&mut self.exposed_properties, server);
        let mut i : i32 = 0;
        while i < self.exposed_properties.carrier_properties.carrying.len() as i32 {
            let clone = self.exposed_properties.carrier_properties.carrying[i as usize].clone();
            let mut lock = clone.lock().await;
            if self.piece.carry_iter(&mut self.exposed_properties, &mut lock.exposed_properties, i as usize) { // drop the carried object
                self.piece.drop_carry(&mut self.exposed_properties, &mut lock.exposed_properties, i as usize);
                self.exposed_properties.carrier_properties.space_remaining += 1;
                lock.exposed_properties.carrier_properties.is_carried = false;
                /*drop(lock);
                self.exposed_properties.carrier_properties.carrying.remove(i as usize);
                i -= 1;*/
            }
            i += 1;
        }
        i = 0;
        while i < self.exposed_properties.carrier_properties.carrying.len() as i32 { // remove everything AFTER they've been released, so reordering doesn't cause problems above
            let clone = self.exposed_properties.carrier_properties.carrying[i as usize].clone();
            let lock = clone.lock().await;
            if !lock.exposed_properties.carrier_properties.is_carried {
                drop(lock);
                self.exposed_properties.carrier_properties.carrying.remove(i as usize);
                i -= 1;
            }
            i += 1;
        }
        if self.exposed_properties.physics.portals {
            self.exposed_properties.physics.set_cx(coterminal(self.exposed_properties.physics.cx(), server.gamesize as f32));
            self.exposed_properties.physics.set_cy(coterminal(self.exposed_properties.physics.cy(), server.gamesize as f32));
        }
        if self.exposed_properties.health_properties.health < self.exposed_properties.health_properties.max_health {
            self.exposed_properties.health_properties.health += self.exposed_properties.health_properties.passive_heal;
        }
        if self.exposed_properties.shooter_properties.shoot {
            if self.exposed_properties.shooter_properties.suppress {
                if self.shoot_timer > 0 {
                    self.shoot_timer -= 1;
                }
            }
            else {
                self.shawty(self.exposed_properties.shooter_properties.range, server).await;
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

    pub async fn shawty(&mut self, range : i32, server : &mut Server) {
        if self.shoot_timer == 0 {
            self.shoot_timer = self.exposed_properties.shooter_properties.counter;
            for angle in &self.exposed_properties.shooter_properties.angles {
                server.shoot(self.exposed_properties.shooter_properties.bullet_type, self.exposed_properties.physics.extend_point(50.0, *angle), Vector2::new_from_manda(20.0, self.exposed_properties.physics.angle() + *angle) + self.exposed_properties.physics.velocity, range, None).await
                .lock().await.set_banner(self.banner); // Set the banner
            }
        }
        else {
            self.shoot_timer -= 1;
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

    pub async fn die(&mut self, server : &mut Server) {
        self.piece.on_die();
        for explosion in &self.exposed_properties.exploder {
            match explosion {
                ExplosionMode::Radiation(size, halflife, strength) => {
                    server.place_radiation(self.exposed_properties.physics.cx(), self.exposed_properties.physics.cy(), *size, *halflife, *strength, self.exposed_properties.physics.angle(), None).await;
                },
                _ => {

                }
            }
        }
        for carried in &self.exposed_properties.carrier_properties.carrying {
            let mut lock = carried.lock().await;
            lock.exposed_properties.carrier_properties.is_carried = false;
            lock.exposed_properties.goal_x = lock.exposed_properties.physics.cx();
            lock.exposed_properties.goal_y = lock.exposed_properties.physics.cy();
            lock.exposed_properties.goal_a = lock.exposed_properties.physics.angle();
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

    pub fn get_new_message(&self) -> ProtocolMessage {
        let mut args = vec![
            self.identify().to_string(),
            self.get_id().to_string(),
            self.exposed_properties.physics.cx().to_string(),
            self.exposed_properties.physics.cy().to_string(),
            self.exposed_properties.physics.angle().to_string(),
            (if self.is_editable() { 1 } else { 0 }).to_string(),
            self.get_banner().to_string(),
            self.exposed_properties.physics.width().to_string(),
            self.exposed_properties.physics.height().to_string()
        ];
        for upg in &self.upgrades {
            args.push(String::clone(&upg));
        }
        ProtocolMessage {
            command: 'n',
            args
        }
    }

    pub fn get_banner(&self) -> usize {
        self.banner
    }

    pub fn set_banner(&mut self, new : usize) {
        self.banner = new;
    }

    pub fn set_id(&mut self, id : u32) {
        self.exposed_properties.id = id;
    }

    pub fn get_id(&self) -> u32 {
        self.exposed_properties.id
    }

    pub fn cost(&self) -> u32 {
        self.piece.cost()
    }

    pub fn capture(&self) -> u32 {
        self.piece.capture()
    }

    pub fn add_fort(&mut self, fort : Arc<Mutex<GamePieceBase>>) {
        self.forts.push(fort);
    }

    pub fn get_max_health(&self) -> f32 {
        self.exposed_properties.health_properties.max_health
    }

    pub fn upgrade(&mut self, up : String) {
        let up = Arc::new(up);
        self.piece.on_upgrade(&mut self.exposed_properties, up.clone());
        self.upgrades.push(up.clone());
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
            PhysicsObject::new(0.0, 0.0, 10.0, 60.0, 0.0)
        }
        else{
            PhysicsObject::new(0.0, 0.0, 50.0, 50.0, 0.0)
        }
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
            id == 'b' || id == 'r' || id == 'h' // All they collide with is bullets and radiation.
        }
    }

    fn capture(&self) -> u32 {
        50
    }

    fn on_upgrade(&mut self, properties : &mut ExposedProperties, upgrade : Arc<String>) {
        println!("{}", upgrade);
        match upgrade.as_str() {
            "b" => { // shot counter speed
                properties.shooter_properties.counter = 9;
            },
            "f" => { // fast
                properties.physics.speed_cap = 40.0;
            },
            "h" => { // heal
                properties.health_properties.passive_heal = 0.007;
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

    fn cost(&self) -> u32 {
        120
    }
}
