use crate::Server;
use crate::ExposedProperties;
use crate::physics::PhysicsObject;
use crate::gamepiece::GamePiece;
use crate::Vector2;
use crate::TargetingMode;
use crate::TargetingFilter;
use crate::functions::coterminal;
use crate::gamepiece::GamePieceBase;


pub struct Nexus {
    place_counter : u16,
    effect_radius : f32,
    players       : Vec<usize> // list of player banners currently in this nexus; refreshed every tick at the moment. OPTIMIZATION PENDING.
}

pub struct NexusEnemy {
    parent    : u32,
    countdown : u16
}

impl Nexus {
    pub fn new(effect_radius : f32) -> Nexus {
        Nexus {
            effect_radius,
            place_counter : 100,
            players: vec![]
        }
    }
}

impl NexusEnemy {
    pub fn new(parent : u32) -> NexusEnemy {
        NexusEnemy {
            parent,
            countdown : 0//300
        }
    }
}

impl GamePiece for Nexus {
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 3.0;
    }

    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 60.0, 60.0, 0.0)
    }

    fn get_does_collide(&self, id : char) -> bool {
        id != 'w'
    }

    fn identify(&self) -> char {
        'N'
    }

    fn on_subscribed_death(&mut self, _me : &mut ExposedProperties, them : &mut GamePieceBase, server : &mut Server) {
        for player in &self.players {
            server.score_to(*player, them.capture() as i32);
        }
    }

    fn update(&mut self, properties : &mut ExposedProperties, server : &mut Server) {
        if properties.health_properties.health <= 0.0 {
            properties.health_properties.health = properties.health_properties.max_health;
            for obj in &mut server.objects {
                if self.players.contains(&obj.get_banner()) && (obj.identify() == 'c' || obj.identify() == 'R') {
                    obj.exposed_properties.health_properties.health = -1.0;
                }
            }
        }
        let big = properties.physics.shape.bigger(self.effect_radius);
        self.players.clear(); // inexpensive operation
        for i in 0..server.objects.len() {
            if server.objects[i].get_banner() != 0 && server.objects[i].identify() != 'b' && !self.players.contains(&server.objects[i].get_banner()) { // if it's owned by a player. this is a cheap single-int check, as opposed to the complex separating-axis-theorem code needed to check intersection with the radius of effect.
                if server.objects[i].exposed_properties.physics.shape.intersects(big).0 {
                    self.players.push(server.objects[i].get_banner());
                }
            }
        }
        if self.players.len() > 0 {
            self.place_counter -= 1;
            if self.place_counter == 0 {
                self.place_counter = 200 + rand::random::<u16>() % 300;
                let pick_pos = coterminal(rand::random::<f32>() * self.effect_radius, self.effect_radius) - self.effect_radius/2.0;
                println!("Pick pos: {}", pick_pos);
                let mut x : f32 = 0.0;
                let mut y : f32 = 0.0;
                match rand::random::<u8>() % 4 {
                    0 => {
                        x = pick_pos;
                        y = -self.effect_radius / 2.0;
                    }
                    1 => {
                        x = pick_pos;
                        y = self.effect_radius / 2.0;
                    }
                    2 => {
                        y = pick_pos;
                        x = -self.effect_radius / 2.0;

                    }
                    3 => {
                        y = pick_pos;
                        x = self.effect_radius / 2.0;
                    }
                    _ => {}
                }
                x += properties.physics.cx();
                y += properties.physics.cy();
                let a = rand::random::<f32>() * std::f32::consts::PI * 2.0;
                let enemy = match rand::random::<u16>() % 4 {
                    0 => {
                        server.place_missile(x, y, a, None)
                    }
                    1 => {
                        server.place_basic_fighter(x, y, a, None)
                    }
                    2 => {
                        server.place_tie_fighter(x, y, a, None)
                    }
                    3 => {
                        server.place_basic_fighter(x, y, a, None)
                    }
                    _ => {0}
                };
                let en_ind = server.obj_lookup(enemy).unwrap();
                server.objects[en_ind].exposed_properties.goal_x = properties.physics.cx();
                server.objects[en_ind].exposed_properties.goal_y = properties.physics.cy();
                server.objects[en_ind].exposed_properties.collision_info.worthit = false;
                server.objects[en_ind].exposed_properties.physics.velocity = Vector2::new_from_manda(20.0, rand::random::<f32>() * std::f32::consts::PI * 2.0);
                server.objects[en_ind].death_subscribe(properties.id);
            }
        }
    }
}


impl GamePiece for NexusEnemy { // DEPRECATION NOTICE: this is no longer in use.
    fn obtain_physics(&self) -> PhysicsObject {
        PhysicsObject::new(0.0, 0.0, 48.0, 20.0, 0.0)
    }

    fn get_does_collide(&self, _id : char) -> bool {
        true
    }
    
    fn construct<'a>(&'a self, thing : &mut ExposedProperties) {
        thing.health_properties.max_health = 1.0;
        thing.collision_info.damage = 3.0;
        thing.targeting.mode = TargetingMode::Id(self.parent);
        thing.targeting.filter = TargetingFilter::Any;
        thing.physics.velocity = Vector2::new_from_manda(10.0, rand::random::<f32>() * std::f32::consts::PI * 2.0);
    }

    fn identify(&self) -> char {
        '&'
    }

    fn update(&mut self, properties : &mut ExposedProperties, _server : &mut Server) {
        match properties.targeting.vector_to {
            Some(goal) => {
                if self.countdown == 0 {
                    properties.physics.set_angle(properties.physics.angle() * 0.9 + goal.angle() * 0.1);
                }
                else {
                    properties.physics.set_angle(properties.physics.angle() * 0.999 + goal.angle() * 0.001);
                }
                let thrust = Vector2::new_from_manda(0.25, properties.physics.angle());
                properties.physics.velocity = properties.physics.velocity + thrust;
                properties.physics.velocity = properties.physics.velocity * 0.99;
            }
            None => {}
        }
        if self.countdown > 0 {
            properties.physics.velocity *= 0.6;
            self.countdown -= 1;
        }
    }
}