// Miscellaneous stuff like bullets and turrets and walls
use super::GamePiece;
use crate::Server;
use crate::physics::PhysicsObject;
use super::TargetingFilter;
use super::TargetingMode;
use super::ExplosionMode;
use crate::ServerToClient;
use crate::vector::Vector2;
use super::BulletType;
use crate::functions::*;
use crate::ExposedProperties;
use crate::ReqZone;
use std::f32::consts::PI;

pub struct Bullet {}
pub struct AntiRTFBullet {}
pub struct Wall {}
pub struct Chest {}
pub struct Turret {}
pub struct MissileLaunchingSystem {}
pub struct GreenThumb {
    countdown : u16
}
pub struct Carrier {
    angle_v : f32
}
pub struct Radiation {
    halflife : f32,
    strength : f32,
    counter  : f32,
    w        : f32,
    h        : f32
}
pub struct Nuke {}
pub struct Block {}
pub struct GoldBar {}
pub struct Seed {
    countdown : u16,
    max_countdown : u16
}
pub struct Air2Air {
    target : u32,
    count  : u32
}


impl Bullet {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl Seed {
    pub fn new() -> Self {
        let countd = 800 + rand::random::<u16>() % 800;
        Self {
            countdown : countd,
            max_countdown : countd
        }
    }
}

impl Carrier {
    pub fn new() -> Self {
        Self {
            angle_v : 0.0
        }
    }
}

impl AntiRTFBullet {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl Wall {
    pub fn new() -> Self {
        Self {}
    }
}

impl Chest {
    pub fn new() -> Self {
        Self {}
    }
}

impl Air2Air {
    pub fn new(tid : u32) -> Self {
        Self {
            target : tid,
            count  : 5
        }
    }
}

impl Turret {
    pub fn new() -> Self {
        Self {}
    }
}

impl MissileLaunchingSystem {
    pub fn new() -> Self {
        Self {}
    }
}

impl Radiation {
    pub fn new(halflife : f32, strength : f32, w : f32, h : f32) -> Self {
        println!("Radiating with halflife {} and strength {}", halflife, strength);
        Self {
            halflife,
            strength,
            counter : 0.0,
            w,
            h
        }
    }
}

impl Nuke {
    pub fn new() -> Self {
        Self {}
    }
}

impl Block {
    pub fn new() -> Self {
        Self {}
    }
}

impl GreenThumb {
    pub fn new() -> Self {
        Self {
            countdown : 10
        }
    }
}

impl GoldBar {
    pub fn new() -> Self {
        Self {}
    }
}


impl GamePiece for GoldBar {
    fn construct(&self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 0.01;
        thing.collision_info.damage = 0.0;
    }

    fn identify(&self) -> char {
        'g'
    }

    fn cost(&self) -> i32 {
        100
    }

    fn capture(&self) -> u32 {
        100
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 50.0, 30.0, 0.0)
    }
}

impl GamePiece for GreenThumb {
    fn construct(&self, thing : &mut ExposedProperties) {
        
    }

    fn update(&mut self, properties : &mut ExposedProperties, server : &mut Server) {
        self.countdown -= 1;
        if self.countdown == 0 {
            self.countdown = (800 + 800) / 30; // there are 20 chests around it, 1200 is the average chest lifetime.
            let vec = properties.physics.vector_position() + Vector2::new_from_manda(200.0, properties.physics.angle());
            server.place_seed(vec.x, vec.y, Some(properties.banner));
            properties.physics.set_angle(properties.physics.angle() + 2.0 * PI / 30.0);
        }
    }

    fn cost(&self) -> i32 {
        1000
    }

    fn capture(&self) -> u32 {
        500
    }

    fn identify(&self) -> char {
        'G'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 20.0, 10.0, 0.0)
    }
}

impl GamePiece for Bullet {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.ttl = 30;
        thing.health_properties.max_health = 1.0;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 10.0, 10.0, 0.0)
    }

    fn identify(&self) -> char {
        'b'
    }
}

impl GamePiece for Carrier {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 1.0;
        thing.health_properties.passive_heal = 0.02;
        thing.collision_info.damage = 1.0;
        thing.physics.speed_cap = 12.0;
        thing.carrier_properties.space_remaining = 10;
        thing.carrier_properties.does_accept = vec!['f', 'h', 's', 't', 'T', 'n', 'm'];
        thing.health_properties.prevent_friendly_fire = true;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 400.0, 160.0, 0.0)
    }

    fn identify(&self) -> char {
        'K'
    }
    
    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        let vec_to = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy());
        let mut thrust = Vector2::new_from_manda(1.0, properties.physics.angle());
        let mut l = loopize(properties.physics.angle(), vec_to.angle());
        if l.abs() > 3.0 * PI/4.0 {
            l = loopize(properties.physics.angle() - PI, vec_to.angle());
            thrust *= -1.0;
        }
        if vec_to.magnitude() < 10.0 {
            l = loopize(properties.physics.angle(), properties.goal_a);
        }
        else {
            properties.physics.velocity = properties.physics.velocity + thrust;
            let mut perp = vec_to.perpendicular();
            let dot = properties.physics.velocity.dot(perp);
            perp.set_magnitude(dot / 2.0);
            properties.physics.velocity += perp * -1.0;
        }
        if loopize(properties.physics.velocity.angle(), vec_to.angle()).abs() > PI / 2.0 { // if it's going in the wrong direction
            properties.physics.velocity *= 0.5; // airbrake
        }
        self.angle_v = -l * 3.0/4.0;
        properties.physics.set_angle(properties.physics.angle() + self.angle_v);
    }

    fn cost(&self) -> i32 {
        80
    }

    fn is_editable(&self) -> bool {
        true
    }

    fn on_carry(&mut self, me : &mut ExposedProperties, thing : &mut ExposedProperties, server : &mut Server) { // when a new object becomes carried by this
        thing.goal_x = -1.0;
        if thing.value == 'h' {
            me.physics.speed_cap += 3.0;
        }
        me.health_properties.max_health += thing.health_properties.max_health;
        me.health_properties.health += thing.health_properties.max_health;
        /* Carrier berths are like
             y
 |    |    |    |    |    |
 | 0  | 2  | 4  | 6  | 8  |
x|----|----|----|----|----|
 | 1  | 3  | 5  | 7  | 9  |
 |    |    |    |    |    |
        */
        let d = thing.physics.vector_position().rotate_about(me.physics.vector_position(), me.physics.angle()) - me.physics.vector_position();
        let mut berthx = if d.y > 0.0 {
            1
        }
        else {
            0
        };
        thing.carrier_properties.berth = berthx;
        // each berth is 80x80
        let mut berthy = ((d.x / 80.0).round() + 2.0) as usize; // rotated coordinate plane
        if berthy > 4 {
            berthy = 4;
        }
        let mut berth_map = [[false; 8]; 2];
        for id in &me.carrier_properties.carrying {
            if *id == thing.id {
                continue;
            }
            match server.obj_lookup(*id) {
                Some (i) => {
                    let berthv = server.objects[i].exposed_properties.carrier_properties.berth;
                    let berthx = berthv % 2;
                    let berthy = berthv / 2;
                    berth_map[berthx][berthy] = true;
                },
                None => {}
            }
        }
        if berth_map[berthx][berthy] {
            let mut best_d = 5;
            let mut best_m = 0;
            for y in 0..5 {
                if !berth_map[berthx][y] {
                    let m = (berthy as i32 - y as i32).abs() as usize;
                    if m < best_d {
                        best_d = m;
                        best_m = y;
                    }
                }
            }
            if best_d < 5 {
                berthy = best_m;
            }
            else {
                for y in 0..5 {
                    if !berth_map[1 - berthx][y] {
                        let m = (berthy as i32 - y as i32).abs() as usize;
                        if m < best_d {
                            best_d = m;
                            best_m = y;
                        }
                    }
                }
                berthx = 1 - berthx;
                berthy = best_m;
            }
        }
        thing.carrier_properties.berth = berthx + berthy * 2;
    }

    fn carry_iter(&mut self, me : &mut ExposedProperties, thing : &mut ExposedProperties) -> bool {
        if !thing.carrier_properties.can_update {
            thing.physics.set_angle(me.physics.angle());
        }
        let berth_y : bool = thing.carrier_properties.berth % 2 == 0;
        let berth_x : usize = thing.carrier_properties.berth / 2; // stupid rust can't handle my u8s so I have to waste a lot of space on a usize for these.
        let mut new_pos = Vector2::new(me.physics.cx() - me.physics.shape.w/2.0 + berth_x as f32 * 80.0 + 35.0, me.physics.cy() - me.physics.shape.h/2.0 + if berth_y { 35.0 } else { me.physics.shape.h - 35.0 });
        new_pos = new_pos.rotate_about(Vector2::new(me.physics.cx(), me.physics.cy()), me.physics.angle());
        thing.physics.set_cx(new_pos.x);
        thing.physics.set_cy(new_pos.y);
        if thing.goal_x != -1.0 {
            return true;
        }
        false
    }

    fn drop_carry(&mut self, me : &mut ExposedProperties, thing : &mut ExposedProperties) {
        let berth_y : bool = thing.carrier_properties.berth % 2 == 0;
        let berth_x : usize = thing.carrier_properties.berth / 2;
        let outsize =  if thing.physics.shape.w > thing.physics.shape.h { thing.physics.shape.w } else { thing.physics.shape.h };
        let mut new_pos = Vector2::new(me.physics.cx() - me.physics.shape.w/2.0 + berth_x as f32 * 80.0 + 35.0, if berth_y { me.physics.shape.y - me.physics.shape.h/2.0 - outsize } else { me.physics.shape.h/2.0 + me.physics.shape.y + outsize });
        new_pos = new_pos.rotate_about(Vector2::new(me.physics.cx(), me.physics.cy()), me.physics.angle());
        thing.physics.set_cx(new_pos.x);
        thing.physics.set_cy(new_pos.y);
        if thing.value == 'h' {
            me.physics.speed_cap -= 3.0;
        }
        me.health_properties.max_health -= thing.health_properties.max_health;
    }
}

impl GamePiece for AntiRTFBullet {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.targeting.mode = TargetingMode::Nearest;
        thing.targeting.filter = TargetingFilter::RealTimeFighter;
        thing.targeting.range = (0.0, 5000.0); // losing these guys is possible, but not easy
        thing.health_properties.max_health = 1.0;
        thing.collision_info.damage = 5.0;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 30.0, 10.0, 0.0)
    }

    fn identify(&self) -> char {
        'a'
    }
    
    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector_to) => {
                let goalangle = vector_to.angle();
                properties.physics.change_angle(loopize(goalangle, properties.physics.angle()) * 0.4);
                if vector_to.magnitude() > 500.0 {
                    properties.physics.thrust(2.0); // go way faster if it's far away
                }
                else {
                    properties.physics.velocity = properties.physics.velocity * 0.99; // add a lil' friction so it can decelerate after going super fast cross-board
                    properties.physics.thrust(1.0); // but also keep some thrust so the angle correction isn't moot
                }
                if (properties.physics.velocity.angle() - goalangle).abs() > PI/3.0 {
                    properties.physics.velocity = properties.physics.velocity * 0.9;
                }
            },
            None => {
                properties.physics.velocity = properties.physics.velocity * 0.8;
            }
        }
    }

    fn cost(&self) -> i32 {
        7
    }
}

impl GamePiece for Air2Air {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.targeting.mode = TargetingMode::Id (self.target);
        thing.targeting.filter = TargetingFilter::Any;
        thing.targeting.range = (0.0, 5000.0); // losing these guys is possible, but not easy
        thing.health_properties.max_health = 1.0;
        thing.collision_info.damage = 5.0;
        thing.ttl = 300;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 30.0, 10.0, 0.0)
    }

    fn identify(&self) -> char {
        'a'
    }
    
    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector_to) => {
                let goalangle = vector_to.angle();
                properties.physics.change_angle(loopize(goalangle, properties.physics.angle()) * 0.2);
                if vector_to.magnitude() > 700.0 {
                    properties.physics.thrust(2.0); // go way faster if it's far away
                }
                else {
                    properties.physics.velocity = properties.physics.velocity * 0.99; // add a lil' friction so it can decelerate after going super fast cross-board
                    properties.physics.thrust(1.0); // but also keep some thrust so the angle correction isn't moot
                }
                if (properties.physics.velocity.angle() - goalangle).abs() > PI/3.0 {
                    properties.physics.velocity = properties.physics.velocity * 0.9;
                }
            },
            None => {
                properties.physics.velocity = properties.physics.velocity * 0.8;
            }
        }
        if self.count > 0 {
            self.count -= 1;
            properties.physics.velocity = properties.physics.velocity * 0.3;
        }
    }

    fn cost(&self) -> i32 {
        0
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::NoZone
    }
}

impl GamePiece for Wall {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 5.0;
        thing.ttl = 2400;
    }

    fn identify(&self) -> char {
        'w'
    }

    fn get_does_collide(&self, thing : char) -> bool {
        thing != 'c' && thing != 'F' && thing != 'B' && thing != 'N' && thing != 'w' // No castles, no forts, no blocks, no nexuses, no other walls
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 60.0, 60.0, 0.0)
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::WithinCastleOrFort
    }

    fn does_grant_a2a(&self) -> bool {
        true
    }
}

impl GamePiece for Seed {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 1.0;
    }

    fn identify(&self) -> char {
        'S'
    }

    fn get_does_collide(&self, _ : char) -> bool {
        true // they collide with literally everything 
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 10.0, 10.0, 0.0)
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::WithinCastleOrFort
    }

    fn cost(&self) -> i32 {
        10
    }

    fn update(&mut self, properties : &mut ExposedProperties, server : &mut Server) {
        self.countdown -= 1;
        if self.countdown <= 0 {
            properties.health_properties.health = -1.0;
            server.place_chest(properties.physics.shape.x, properties.physics.shape.y, None);
        }
        server.send_to(ServerToClient::SeedCompletion (properties.id, ((self.countdown as u32 * 100) / (self.max_countdown as u32)) as u16), properties.banner);
    }
}

impl GamePiece for Chest {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 2.0;
        thing.ttl = 4800;
    }

    fn identify(&self) -> char {
        'C'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 30.0, 30.0, 0.0)
    }

    fn get_does_collide(&self, thing : char) -> bool {
        thing != 'c' && thing != 'F' && thing != 'B' && thing != 'S' // No castles, no forts, no blocks, no seeds
    }

    fn capture(&self) -> u32 {
        50
    }
}

impl GamePiece for Turret {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.targeting.mode = TargetingMode::Nearest;
        thing.targeting.filter = TargetingFilter::Fighters;
        thing.targeting.range = (0.0, 500.0);
        thing.shooter_properties.shoot = true;
        thing.shooter_properties.counter = 30;
        thing.carrier_properties.can_update = true;
    }

    fn identify(&self) -> char {
        'T'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 48.0, 22.0, 0.0)
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::WithinCastleOrFort
    }

    fn update(&mut self, properties : &mut ExposedProperties, server : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector) => {
                properties.physics.set_angle(vector.angle());
                properties.shooter_properties.suppress = false;
                if properties.carrier_properties.is_carried {
                    properties.shooter_properties.suppress = false;
                }
            },
            None => {
                if properties.carrier_properties.is_carried {
                    properties.shooter_properties.suppress = true;
                }
            }
        };
    }

    fn cost(&self) -> i32 {
        100
    }
}

impl GamePiece for MissileLaunchingSystem {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.targeting.mode = TargetingMode::Nearest;
        thing.targeting.filter = TargetingFilter::RealTimeFighter;
        thing.targeting.range = (0.0, 1000.0);
        thing.shooter_properties.shoot = true;
        thing.shooter_properties.bullet_type = BulletType::AntiRTF;
        thing.shooter_properties.counter = 150;
        thing.shooter_properties.range = 1000;
        thing.collision_info.damage = 5.0;
        thing.carrier_properties.can_update = true;
    }

    fn identify(&self) -> char {
        'm'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 48.0, 22.0, 0.0)
    }

    fn req_zone(&self) -> ReqZone {
        ReqZone::WithinCastleOrFort
    }

    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector) => {
                properties.physics.set_angle(vector.angle());
                properties.shooter_properties.suppress = false;
            },
            None => {
                properties.shooter_properties.suppress = true; // don't fire when there isn't a target, these guys are supposed to be more convenient than regular turrets
            }
        };
    }

    fn cost(&self) -> i32 {
        100
    }
}

impl GamePiece for Radiation {
    fn identify(&self) -> char {
        'r'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, self.w, self.h, 0.0)
    }

    fn update(&mut self, properties : &mut ExposedProperties, server : &mut Server) {
        let strength = (0.5_f32).powf(self.counter/self.halflife) * self.strength;
        self.counter += 1.0;
        properties.collision_info.damage = strength/12.0;
        server.broadcast(ServerToClient::Radiate (properties.id, strength));
        if strength < 0.01 {
            properties.health_properties.health = 0.0;
        }
    }

    fn get_does_collide(&self, _id : char) -> bool {
        false
    }
}

impl GamePiece for Nuke {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.exploder = vec![
            ExplosionMode::Radiation(200.0, 60.0, 0.3),
            ExplosionMode::Radiation(1500.0, 250.0, 0.3),
            ExplosionMode::Radiation(6000.0, 700.0, 0.3)
        ];
        thing.collision_info.damage = 0.0;
        thing.ttl = 500;
    }

    fn identify(&self) -> char {
        'n'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 36.0, 36.0, 0.0)
    }

    fn cost(&self) -> i32 {
        300
    }

    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        let thrust = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy()).unit() * 0.1;
        properties.physics.velocity = properties.physics.velocity + thrust;
        properties.physics.velocity = properties.physics.velocity * 0.999;
    }

    fn is_editable(&self) -> bool {
        true
    }
}

impl GamePiece for Block {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.physics.mass *= 1.0;//100.0; // Very high density: inexorable push
        thing.collision_info.damage = 0.0; // Does no collision damage
        thing.physics.solid = true;
        thing.health_properties.max_health = 1000.0;
        thing.physics.fixed = true;
    }

    fn identify(&self) -> char {
        'B'
    }

    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        properties.health_properties.health = properties.health_properties.max_health; // it cannot die
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 300.0, 300.0, 0.0)
    }
}