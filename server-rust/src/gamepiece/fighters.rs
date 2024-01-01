// Grand unified file for fighter types

use super::GamePiece;
use crate::Server;
use crate::physics::PhysicsObject;
use crate::vector::Vector2;
use crate::ExposedProperties;
use crate::BulletType;
use crate::ExplosionMode;


pub struct BasicFighter {}
pub struct TieFighter {}
pub struct Sniper {}
pub struct Missile {}
pub struct Artillery {}


impl BasicFighter {
    pub fn new() -> Self {
        Self {}
    }
}

impl TieFighter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Sniper {
    pub fn new() -> Self {
        Self {}
    }
}

impl Missile {
    pub fn new() -> Self {
        Self {}
    }
}

impl Artillery {
    pub fn new() -> Self {
        Self {}
    }
}


impl GamePiece for BasicFighter {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.shooter_properties.shoot = true;
        thing.shooter_properties.counter = 30;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 48.0, 36.0, 0.0)
    }

    fn identify(&self) -> char {
        'f'
    }

    fn get_does_collide(&self, id : char) -> bool {
        id != 'c'
    }

    fn cost(&self) -> i32 {
        10
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        let mut thrust = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy());
        if thrust.magnitude() < 10.0 {
            properties.physics.set_angle(properties.goal_a);
        }
        else {
            thrust = thrust.unit() * 0.25;
            properties.physics.set_angle(thrust.angle());
            properties.physics.velocity = properties.physics.velocity + thrust;
        }
        properties.physics.velocity = properties.physics.velocity * 0.95;
    }

    fn is_editable(&self) -> bool {
        true
    }

    fn on_upgrade(&mut self, properties : &mut ExposedProperties, upgrade : &String) {
        match upgrade.as_str() {
            "laser" => {
                properties.shooter_properties.bullet_type = BulletType::Laser (0.3, 50000.0);
                properties.shooter_properties.counter = 1;
            },
            _ => {}
        }
    }
}

impl GamePiece for TieFighter {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.shooter_properties.shoot = true;
        thing.shooter_properties.counter = 40;
        thing.repeater.max_repeats = 1;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 32.0, 36.0, 0.0)
    }

    fn identify(&self) -> char {
        't'
    }

    fn get_does_collide(&self, id : char) -> bool {
        id != 'c'
    }

    fn cost(&self) -> i32 {
        20
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        let mut thrust = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy());
        if thrust.magnitude() < 10.0 {
            properties.physics.set_angle(properties.goal_a);
        }
        else {
            thrust = thrust.unit() * 0.35;
            properties.physics.set_angle(thrust.angle());
            properties.physics.velocity = properties.physics.velocity + thrust;
        }
        properties.physics.velocity = properties.physics.velocity * 0.95;
    }

    fn is_editable(&self) -> bool {
        true
    }
}

impl GamePiece for Sniper {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.shooter_properties.shoot = true;
        thing.shooter_properties.counter = 80;
        thing.shooter_properties.range = 90;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 72.0, 20.0, 0.0)
    }

    fn identify(&self) -> char {
        's'
    }

    fn get_does_collide(&self, id : char) -> bool {
        id != 'c'
    }

    fn cost(&self) -> i32 {
        30
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        let mut thrust = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy());
        if thrust.magnitude() < 10.0 {
            properties.physics.set_angle(properties.goal_a);
        }
        else {
            thrust = thrust.unit() * 1.2;
            properties.physics.set_angle(thrust.angle());
            properties.physics.velocity = properties.physics.velocity + thrust;
        }
        properties.physics.velocity = properties.physics.velocity * 0.9;
    }

    fn is_editable(&self) -> bool {
        true
    }
}

impl GamePiece for Missile {
    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 48.0, 20.0, 0.0)
    }

    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.collision_info.damage = 0.5;
        thing.health_properties.max_health = 0.5; // collisions with themselves will detonate both. collisions with other things do 2 damage to the other thing - 0.5 from the collison, 1.5 from the blast.
        thing.exploder = vec![
            ExplosionMode::Blast(100.0, 1.5, 400.0)
        ];
    }

    fn identify(&self) -> char {
        'h'
    }

    fn get_does_collide(&self, _id : char) -> bool {
        true
    }

    fn cost(&self) -> i32 {
        5
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        let goal = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy());
        properties.physics.set_angle(properties.physics.angle() * 0.9 + goal.angle() * 0.1);
        let thrust = Vector2::new_from_manda(0.3, properties.physics.angle());
        properties.physics.velocity = properties.physics.velocity + thrust;
        properties.physics.velocity = properties.physics.velocity * 0.99;
    }

    fn is_editable(&self) -> bool {
        true
    }
}

impl GamePiece for Artillery {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.shooter_properties.shoot = true;
        thing.shooter_properties.bullet_type = BulletType::Mortar (200.0, 0.0, 600.0);
        thing.shooter_properties.counter = 100;
        thing.shooter_properties.range = 120;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 20.0, 50.0, 0.0)
    }

    fn identify(&self) -> char {
        'A'
    }

    fn get_does_collide(&self, id : char) -> bool {
        id != 'c'
    }

    fn cost(&self) -> i32 {
        50
    }

    fn update(&mut self, properties : &mut ExposedProperties, _servah : &mut Server) {
        let mut thrust = Vector2::new(properties.goal_x - properties.physics.cx(), properties.goal_y - properties.physics.cy());
        if thrust.magnitude() < 10.0 {
            properties.physics.set_angle(properties.goal_a);
        }
        else {
            thrust = thrust.unit() * 0.1;
            properties.physics.set_angle(thrust.angle());
            properties.physics.velocity = properties.physics.velocity + thrust;
        }
        properties.physics.velocity = properties.physics.velocity * 0.95;
    }

    fn is_editable(&self) -> bool {
        true
    }
}