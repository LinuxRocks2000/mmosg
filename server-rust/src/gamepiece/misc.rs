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
pub struct Carrier {}
pub struct Radiation {
    halflife : f32,
    strength : f32,
    counter  : f32,
    w        : f32,
    h        : f32
}
pub struct Nuke {}
pub struct Block {}
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

impl Carrier {
    pub fn new() -> Self {
        Self {

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
        thing.physics.speed_cap = 3.0;
        thing.carrier_properties.space_remaining = 10;
        thing.carrier_properties.does_accept = vec!['f', 'h', 's', 't'];
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 400.0, 160.0, 0.0)
    }

    fn identify(&self) -> char {
        'K'
    }
    
    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        let mut thrust = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy());
        if thrust.magnitude() < 10.0 {
            properties.physics.set_angle(properties.goal_a);
            properties.physics.velocity = properties.physics.velocity * 0.7; // airbrake
        }
        else {
            thrust = thrust.unit() * 0.05;
            properties.physics.set_angle(thrust.angle());
            properties.physics.velocity = properties.physics.velocity + thrust;
        }
    }

    fn cost(&self) -> i32 {
        80
    }

    fn is_editable(&self) -> bool {
        true
    }

    fn on_carry(&mut self, me : &mut ExposedProperties, thing : &mut ExposedProperties) { // when a new object becomes carried by this
        thing.goal_x = -1.0;
        if thing.value == 'h' {
            me.physics.speed_cap += 1.0;
        }
        me.health_properties.max_health += thing.health_properties.max_health;
        me.health_properties.health += thing.health_properties.max_health;
    }

    fn carry_iter(&mut self, me : &mut ExposedProperties, thing : &mut ExposedProperties, berth : usize) -> bool {
        thing.physics.set_angle(me.physics.angle());
        let berth_y : bool = berth % 2 == 0;
        let berth_x : usize = berth / 2; // stupid rust can't handle my u8s so I have to waste a lot of space on a usize for these.
        let mut new_pos = Vector2::new(me.physics.cx() - me.physics.shape.w/2.0 + berth_x as f32 * 80.0 + 35.0, me.physics.cy() - me.physics.shape.h/2.0 + if berth_y { 35.0 } else { me.physics.shape.h - 35.0 });
        new_pos = new_pos.rotate_about(Vector2::new(me.physics.cx(), me.physics.cy()), me.physics.angle());
        thing.physics.set_cx(new_pos.x);
        thing.physics.set_cy(new_pos.y);
        if thing.goal_x != -1.0 {
            return true;
        }
        false
    }

    fn drop_carry(&mut self, me : &mut ExposedProperties, thing : &mut ExposedProperties, berth : usize) {
        let berth_y : bool = berth % 2 == 0;
        let berth_x : usize = berth / 2;
        let outsize =  if thing.physics.shape.w > thing.physics.shape.h { thing.physics.shape.w } else { thing.physics.shape.h };
        let mut new_pos = Vector2::new(me.physics.cx() - me.physics.shape.w/2.0 + berth_x as f32 * 80.0 + 35.0, if berth_y { me.physics.shape.y - me.physics.shape.h/2.0 - outsize } else { me.physics.shape.h/2.0 + me.physics.shape.y + outsize });
        new_pos = new_pos.rotate_about(Vector2::new(me.physics.cx(), me.physics.cy()), me.physics.angle());
        thing.physics.set_cx(new_pos.x);
        thing.physics.set_cy(new_pos.y);
        if thing.value == 'h' {
            me.physics.speed_cap -= 1.0;
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
        thing.health_properties.max_health = 2.0;
        thing.ttl = 1800;
    }

    fn identify(&self) -> char {
        'w'
    }

    fn get_does_collide(&self, thing : char) -> bool {
        thing != 'c' && thing != 'F' && thing != 'B' // No castles, no forts, no blocks
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 30.0, 30.0, 0.0)
    }

    fn does_grant_a2a(&self) -> bool {
        true
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
        thing != 'c' && thing != 'F' && thing != 'B' // No castles, no forts, no blocks
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
    }

    fn identify(&self) -> char {
        'T'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 48.0, 22.0, 0.0)
    }

    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector) => {
                properties.physics.set_angle(vector.angle());
            },
            None => {}
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
    }

    fn identify(&self) -> char {
        'm'
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 48.0, 22.0, 0.0)
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