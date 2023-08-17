// Grand unified file for non player character types

use super::GamePiece;
use crate::Server;
use crate::physics::PhysicsObject;
use super::TargetingFilter;
use super::TargetingMode;
use crate::vector::Vector2;
use crate::ExposedProperties;
use std::f32::consts::PI;
use crate::functions::*;

pub struct Red {
    start_cooldown : u32
}

pub struct White {
    start_cooldown : u32
}

pub struct Black {
    start_cooldown : u32
}

pub struct Target {
    count : u32
}

impl Red {
    pub fn new() -> Self {
        Self {
            start_cooldown : 120
        }
    }
}

impl White {
    pub fn new() -> Self {
        Self {
            start_cooldown : 120
        }
    }
}

impl Black {
    pub fn new() -> Self {
        Self {
            start_cooldown : 0
        }
    }
}

impl Target {
    pub fn new() -> Self {
        Self {
            count : 0
        }
    }
}

impl GamePiece for Red {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.targeting.mode = TargetingMode::Nearest;
        thing.targeting.filter = TargetingFilter::Castles;
        thing.physics.speed_cap = 20.0;
        thing.targeting.range = (0.0, 0.0); // no range
        thing.ttl = 1800;
        thing.health_properties.max_health = 1.0;
        thing.collision_info.damage = 2.0;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 20.0, 20.0, 0.0)
    }

    fn identify(&self) -> char {
        '0'
    }

    fn get_does_collide(&self, _id : char) -> bool {
        true
    }

    fn capture(&self) -> u32 {
        5
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector_to) => {
                let goalangle = vector_to.angle();
                properties.physics.change_angle(loopize(goalangle, properties.physics.angle()) * 0.14);
                properties.physics.thrust(0.2);
            },
            None => {}
        }
        if self.start_cooldown != 0 {
            self.start_cooldown -= 1;
            properties.physics.velocity = properties.physics.velocity * 0.8;
        }
    }
}

impl GamePiece for White {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.targeting.mode = TargetingMode::Nearest;
        thing.targeting.filter = TargetingFilter::Fighters;
        thing.targeting.range = (0.0, 0.0); // no range
        thing.physics.speed_cap = 15.0;
        thing.ttl = 1800;
        thing.health_properties.max_health = 1.0;
        thing.collision_info.damage = 2.0;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 20.0, 20.0, 0.0)
    }

    fn identify(&self) -> char {
        '1'
    }

    fn get_does_collide(&self, _id : char) -> bool {
        true
    }

    fn capture(&self) -> u32 {
        5
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector_to) => {
                let goalangle = vector_to.angle();
                properties.physics.change_angle(loopize(goalangle, properties.physics.angle()) * 0.25);
                properties.physics.thrust(0.2);
            },
            None => {}
        }
        if self.start_cooldown != 0 {
            self.start_cooldown -= 1;
            properties.physics.velocity = properties.physics.velocity * 0.8;
        }
    }
}

impl GamePiece for Black {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.targeting.mode = TargetingMode::Nearest;
        thing.targeting.filter = TargetingFilter::Castles;
        thing.targeting.range = (0.0, 0.0); // no range
        thing.ttl = 1800;
        thing.health_properties.max_health = 1.0;
        thing.physics.speed_cap = 35.0; // Very slightly slower than a speedship
        thing.collision_info.damage = 2.0;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 20.0, 20.0, 0.0)
    }

    fn identify(&self) -> char {
        '2'
    }

    fn get_does_collide(&self, id : char) -> bool {
        id == 'c' || id == 'R' || id == 'b'
    }

    fn capture(&self) -> u32 {
        5
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        match properties.targeting.vector_to {
            Some(vector_to) => {
                let goalangle = vector_to.angle();
                properties.physics.change_angle(loopize(goalangle, properties.physics.angle()) * 0.2);
                properties.physics.thrust(1.0);
                if (properties.physics.velocity.angle() - goalangle).abs() > PI/4.0 {
                    properties.physics.velocity = properties.physics.velocity * 0.95;
                }
            },
            None => {}
        }
        if self.start_cooldown != 0 {
            self.start_cooldown -= 1;
            properties.physics.velocity = properties.physics.velocity * 0.8;
        }
    }
}

impl GamePiece for Target {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.ttl = 1800;
        thing.health_properties.max_health = 3.0;
        thing.physics.speed_cap = 10.0;
        thing.collision_info.damage = 0.1; // inoffensive
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 40.0, 40.0, 0.0)
    }

    fn identify(&self) -> char {
        '3'
    }

    fn get_does_collide(&self, id : char) -> bool {
        id == 'b' // is unaffected by everything but bullets
    }

    fn capture(&self) -> u32 {
        30
    }

    fn update(&mut self, properties : &mut ExposedProperties, server : &mut Server) {
        if properties.physics.velocity.magnitude() != properties.physics.speed_cap || self.count == 0 {
            let item = rand::random::<f32>() * PI * 2.0;
            properties.physics.velocity = Vector2::new_from_manda(properties.physics.speed_cap, item);
            self.count = 60;
        }
        self.count -= 1;
        properties.physics.set_cx(coterminal(properties.physics.cx(), server.gamesize as f32));
        properties.physics.set_cy(coterminal(properties.physics.cy(), server.gamesize as f32));
    }
}