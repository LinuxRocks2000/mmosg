/*
    By Tyler Clarke
*/

#![allow(non_camel_case_types)]
// Warp-based rewrite of the server.
pub mod vector;
pub mod physics;
pub mod gamepiece;
pub mod config;
pub mod functions;
use crate::vector::Vector2;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use warp::Filter;
use warp::ws::{Message, WebSocket};
use std::vec::Vec;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::gamepiece::*;
use std::fmt;
use std::f32::consts::PI;
use rand::Rng;
use crate::gamepiece::fighters::*;
use crate::gamepiece::misc::*;
use crate::gamepiece::npc;
use crate::physics::BoxShape;
use crate::config::Config;
use futures::future::FutureExt; // for `.fuse()`
use std::collections::HashMap;
use tokio::select;
use crate::gamepiece::BulletType;

const FPS : f32 = 30.0;


#[derive(PartialEq, Copy, Clone, Debug)]
enum ClientMode {
    None,
    Normal,
    Defense,
    RealTimeFighter
}


pub struct Client {
    tx                : SplitSink<WebSocket, Message>,
    is_authorized     : bool,
    score             : i32,
    has_placed        : bool,
    banner            : usize,
    m_castle          : Option<u32>,
    mode              : ClientMode,
    team              : Option<usize>,
    commandah         : tokio::sync::mpsc::Sender<ServerCommand>,
    is_team_leader    : bool,
    places_this_turn  : u8,
    kys               : bool,
    a2a               : u16
}


#[derive(PartialEq, Copy, Clone, Debug)]
enum GameMode {
    Waiting,  // The game hasn't started yet: you can join at this point. Countdown may exist.
    Strategy, // Strategy change
    Play,     // Ships are moving
}

struct TeamData {
    id               : usize,
    banner_id        : usize,
    password         : Arc<String>,
    members          : Vec <usize>, // BANNERS
    live_count       : u32        // how many people are actually flying under this team's banner
}


#[derive(Debug, Clone)]
enum ClientCommand { // Commands sent to clients
    Send (ProtocolMessage),
    //SendToTeam (ProtocolMessage, usize),
    Tick (u32, String),
    ScoreTo (usize, i32),
    CloseAll,
    ChatRoom (String, usize, u8, Option<usize>), // message, sender, priority
    GrantA2A (usize),
    AttachToBanner (u32, usize, bool)
}


pub struct Server {
    mode              : GameMode,
    password          : String,
    objects           : Vec<GamePieceBase>,
    teams             : Vec<TeamData>,
    banners           : Vec<Arc<String>>,
    gamesize          : f32,
    authenticateds    : u32,
    terrain_seed      : u32,
    top_id            : u32,
    counter           : u32,
    costs             : bool, // Whether or not to cost money when placing a piece
    place_timer       : u32,
    autonomous        : Option<(u32, u32, u32, u32)>,
    is_io             : bool, // IO mode gets rid of the winner system (game never ends) and allows people to join at any time.
    passwordless      : bool,
    config            : Option<Arc<Config>>,
    broadcast_tx      : tokio::sync::broadcast::Sender<ClientCommand>,
    living_players    : u32,
    winning_banner    : usize,
    isnt_rtf          : u32,
    times             : (f32, f32),
    clients_connected : u32,
    is_headless       : bool,
    permit_npcs       : bool,
    port              : u16,
    sql               : String,
    upg_costs         : HashMap<String, i32>
}

enum AuthState {
    Error,
    Single,
    Team (usize, bool),
    Spectator
}

impl Server {
    fn new_user_can_join(&self) -> bool {
        let mut moidah = self.mode == GameMode::Waiting;
        if self.is_io {
            moidah = true; // Waiting means nothing during io mode.
        }
        if self.autonomous.is_some() {
            moidah = moidah && (self.authenticateds < self.autonomous.unwrap().1);
        }
        return moidah;
    }

    fn object_field_check(&self, object : BoxShape, x : f32, y : f32) -> bool {
        object.ong_fr().bigger(800.0).contains(Vector2::new(x, y)) // this ong frs it first because contains doesn't really work on rotated objects
    }

    fn is_inside_friendly(&self, x : f32, y : f32, banner : usize, tp : char) -> bool {
        for obj in &self.objects {
            if obj.identify() == tp {
                if obj.get_banner() == banner {
                    if self.object_field_check(obj.exposed_properties.physics.shape, x, y) {
                        return true; // short circuit
                    }
                }
            }
        }
        return false;
    }

    fn is_clear(&self, x : f32, y : f32) -> bool {
        for obj in &self.objects {
            if self.object_field_check(obj.exposed_properties.physics.shape, x, y) {
                return false; // short circuit
            }
        }
        return true;
    }

    fn place(&mut self, piece : Box<dyn GamePiece + Send + Sync>, x : f32, y : f32, a : f32, banner : Option<usize>) -> u32 { // return the id of the object released
        let zone = piece.req_zone();
        let la_thang = GamePieceBase::new(piece, x, y, a);
        if banner.is_some() {
            let banner = banner.unwrap();
            if !match zone {
                ReqZone::NoZone => true,
                ReqZone::WithinCastleOrFort => {
                    self.is_inside_friendly(x, y, banner, 'c') || self.is_inside_friendly(x, y, banner, 'F') || self.is_inside_friendly(x, y, banner, 'R')
                },
                ReqZone::AwayFromThings => {
                    self.is_clear(x, y)
                },
                ReqZone::Both => {
                    self.is_clear(x, y) || self.is_inside_friendly(x, y, banner, 'c') || self.is_inside_friendly(x, y, banner, 'F')
                }
            } {
                //sender.as_mut().unwrap().kys = true; // drop the client, something nefarious is going on
                return 0; // refuse to place, returning nothing.
            }
            /*if self.costs {
                let cost = la_thang.cost() as i32;
                if cost > sender.as_ref().unwrap().score {
                    return 0;
                }
                self.broadcast_tx.send(ClientCommand::ScoreTo (banner, -cost)).expect("BROADCAST FAILED");
            }*/
        }
        self.add(la_thang, banner)
    }

    fn obj_lookup(&self, id : u32) -> Option<usize> { // GIVEN the ID of an OBJECT, return the INDEX or NONE if it DOES NOT EXIST.
        for i in 0..self.objects.len() {
            if self.objects[i].get_id() == id {
                return Some(i);
            }
        }
        return None;
    }

    fn place_wall(&mut self, x : f32, y : f32, sender : Option<usize>) {
        self.place(Box::new(Wall::new()), x, y, 0.0, sender);
    }

    fn place_castle(&mut self, x : f32, y : f32, is_rtf : bool, sender : Option<usize>) -> u32 {
        self.place(Box::new(Castle::new(is_rtf)), x, y, 0.0, sender)
    }

    fn place_basic_fighter(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(BasicFighter::new()), x, y, a, sender)
    }

    fn place_block(&mut self, x : f32, y : f32, a : f32, w : f32, h : f32) { // No sender; blocks can't be placed by clients.
        let id = self.place(Box::new(Block::new()), x, y, a, None);
        let i = self.obj_lookup(id).expect("SOMETHING WENT TERRIBLY WRONG"); // in this case the object is guaranteed to exist by the time the lookup is performed, so unwrapping directly is safe.
        self.objects[i].exposed_properties.physics.shape.w = w; // TODO: make this nicer
        self.objects[i].exposed_properties.physics.shape.h = h;
        self.objects[i].exposed_properties.physics.set_cx(x);
        self.objects[i].exposed_properties.physics.set_cy(y);
    }

    fn place_tie_fighter(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(TieFighter::new()), x, y, a, sender)
    }

    fn place_sniper(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Sniper::new()), x, y, a, sender)
    }

    fn place_missile(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Missile::new()), x, y, a, sender)
    }

    fn place_turret(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Turret::new()), x, y, a, sender)
    }

    fn place_mls(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(MissileLaunchingSystem::new()), x, y, a, sender)
    }

    fn place_antirtf_missile(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(AntiRTFBullet::new()), x, y, a, sender)
    }

    fn place_carrier(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Carrier::new()), x, y, a, sender)
    }

    fn place_radiation(&mut self, x : f32, y : f32, size : f32, halflife : f32, strength : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Radiation::new(halflife, strength, size, size)), x, y, a, sender)
    }

    fn place_nuke(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Nuke::new()), x, y, a, sender)
    }

    fn place_fort(&mut self, x : f32, y : f32, a : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Fort::new()), x, y, a, sender)
    }

    fn place_air2air(&mut self, x : f32, y : f32, a : f32, target : u32, sender : Option<usize>) -> u32 {
        self.place(Box::new(Air2Air::new(target)), x, y, a, sender)
    }

    fn place_random_npc(&mut self) { // Drop a random npc
        if self.living_players == 0 || self.isnt_rtf > 0 { // All RTF games will spawn NPCs
            return;
        }
        let x = rand::random::<f32>() % self.gamesize;
        let y = rand::random::<f32>() % self.gamesize;
        let chance = rand::random::<u8>() % 6;
        let thing : Box<dyn GamePiece + Send + Sync> = match chance {
            0 | 1 => {
                Box::new(npc::Red::new())
            },
            2 | 3 => {
                Box::new(npc::White::new())
            },
            4 => {
                Box::new(npc::Black::new())
            },
            5 => {
                Box::new(npc::Target::new())
            },
            _ => {
                println!("THIS IS PROBABLY NOT A GOOD THING");
                return;
            }
        };
        for object in &self.objects {
            if object.identify() == 'c' || object.identify() == 'R' {
                if (object.exposed_properties.physics.cx() - x).abs() < 400.0 && (object.exposed_properties.physics.cy() - y).abs() < 400.0 {
                    println!("Berakx");
                    return;
                }
            }
        }
        self.place(thing, x, y, 0.0, None);
    }

    fn place_random_rubble(&mut self) { // Drop a random chest or wall (or something else, if I add other things)
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0.0..self.gamesize);
        let y = rng.gen_range(0.0..self.gamesize);
        let chance = rand::random::<u8>() % 100;
        let thing : Box<dyn GamePiece + Send + Sync> = {
            if chance < 50 {
                Box::new(Chest::new())
            }
            else {
                Box::new(Wall::new())
            }
        };
        for object in &self.objects {
            if object.identify() == 'c' || object.identify() == 'R' {
                if (object.exposed_properties.physics.cx() - x).abs() < 400.0 && (object.exposed_properties.physics.cy() - y).abs() < 400.0 {
                    println!("Berakx");
                    return;
                }
            }
        }
        self.place(thing, x, y, 0.0, None);
        if self.permit_npcs {
            self.place_random_npc();
        }
    }

    pub fn shoot(&mut self, bullet_type : BulletType, position : Vector2, velocity : Vector2, range : i32, sender : Option<usize>) -> u32 {
        let bullet = self.place(match bullet_type {
            BulletType::Bullet => Box::new(Bullet::new()),
            BulletType::AntiRTF => Box::new(AntiRTFBullet::new())
        }, position.x, position.y, velocity.angle(), sender);
        let i = self.obj_lookup(bullet).unwrap(); // it can be safely unwrapped because the object is guaranteed to exist at this point
        self.objects[i].exposed_properties.physics.velocity = velocity; // TODO: make this nicer
        self.objects[i].exposed_properties.ttl = range;
        bullet
    }

    fn carry_tasks(&mut self, carrier : usize, carried : usize) { // expects that you've already done the lookups - this is the result of a very effective premature optimization in the physics engine
        let carry_id = self.objects[carried].get_id();
        self.objects[carrier].exposed_properties.carrier_properties.carrying.push(carry_id);
        self.objects[carrier].exposed_properties.carrier_properties.space_remaining -= 1;
        self.objects[carried].exposed_properties.carrier_properties.is_carried = true;
        let mut carrier_props = self.objects[carrier].exposed_properties.clone();
        let mut carried_props = self.objects[carried].exposed_properties.clone();
        self.objects[carrier].piece.on_carry(&mut carrier_props, &mut carried_props);
        self.objects[carrier].exposed_properties = carrier_props;
        self.objects[carried].exposed_properties = carried_props;
        // NOTE: If this doesn't work because of the borrow checker mad at having 2 (3???) mutable references to self.objects, just use copying on the ExposedProperties!
        // Since carrying is a relatively rare operation, the wastefulness is not significant.
    }

    fn deal_with_objects(&mut self) {
        if self.objects.len() == 0 {
            return;
        }
        for x in 0..(self.objects.len() - 1) { // Go from first until the next-to-last item, because the inner loop goes from second to last.
            for y in (x + 1)..self.objects.len() {
                if x == y {
                    println!("HANGING BECAUSE X EQUALS Y! CHECK YOUR MATH!");
                }
                if self.objects[x].exposed_properties.carrier_properties.is_carried || self.objects[y].exposed_properties.carrier_properties.is_carried {
                    continue; // Never do any kind of collisions on carried objects.
                }
                let intasectah = self.objects[x].exposed_properties.physics.shape().intersects(self.objects[y].exposed_properties.physics.shape());
                if intasectah.0 {
                    if self.objects[x].exposed_properties.carrier_properties.will_carry(self.objects[y].identify()) {
                        self.carry_tasks(x, y);
                        continue;
                    }
                    if self.objects[y].exposed_properties.carrier_properties.will_carry(self.objects[x].identify()) {
                        self.carry_tasks(y, x);
                        continue;
                    }
                    let mut is_collide = false;
                    if self.objects[x].get_does_collide(self.objects[y].identify()) {
                        let dmg = self.objects[y].get_collision_info().damage;
                        self.objects[x].damage(dmg);
                        if self.objects[x].dead() && (self.objects[y].get_banner() != self.objects[x].get_banner()) {
                            /*let killah = self.get_client_by_banner(self.objects[y].get_banner()).await;
                            if killah.is_some() {
                                let amount = self.objects[x].capture().await as i32;
                                killah.unwrap().lock().await.collect(amount).await;
                            }*/
                            self.broadcast_tx.send(ClientCommand::ScoreTo (self.objects[y].get_banner(), self.objects[x].capture() as i32)).expect("Broadcast failed");
                            if self.objects[x].does_grant_a2a() {
                                self.broadcast_tx.send(ClientCommand::GrantA2A (self.objects[y].get_banner())).expect("Broadcast failed part 2");
                            }
                        }
                        is_collide = true;
                    }
                    if self.objects[y].get_does_collide(self.objects[x].identify()) {
                        let dmg = self.objects[x].get_collision_info().damage;
                        self.objects[y].damage(dmg);
                        if self.objects[y].dead() && (self.objects[y].get_banner() != self.objects[x].get_banner()) {
                            /*let killah = self.get_client_by_banner(self.objects[x].get_banner()).await;
                            if killah.is_some() {
                                let amount = self.objects[y].capture().await as i32;
                                killah.unwrap().lock().await.collect(amount).await;
                            }*/
                            self.broadcast_tx.send(ClientCommand::ScoreTo (self.objects[x].get_banner(), self.objects[y].capture() as i32)).expect("Broadcast failed");
                            if self.objects[y].does_grant_a2a() {
                                self.broadcast_tx.send(ClientCommand::GrantA2A (self.objects[x].get_banner())).expect("Broadcast failed part 2");
                            }
                        }
                        is_collide = true;
                    }
                    if is_collide {
                        if self.objects[x].exposed_properties.physics.solid || self.objects[y].exposed_properties.physics.solid {
                            let sum = self.objects[y].exposed_properties.physics.velocity.magnitude() + self.objects[x].exposed_properties.physics.velocity.magnitude();
                            let ratio = if sum == 0.0 {
                                //self.objects[y].physics.mass / (self.objects[x].physics.mass + self.objects[y].physics.mass)
                                if self.objects[y].exposed_properties.physics.mass < self.objects[x].exposed_properties.physics.mass {
                                    0.0
                                } else {
                                    1.0
                                }
                            }
                            else {
                                self.objects[x].exposed_properties.physics.velocity.magnitude() / sum
                            };
                            if !self.objects[x].exposed_properties.physics.fixed {
                                self.objects[x].exposed_properties.physics.shape.translate(intasectah.1 * ratio); // I have no clue if this is correct but it works well enough
                            }
                            if !self.objects[y].exposed_properties.physics.fixed {
                                self.objects[y].exposed_properties.physics.shape.translate(intasectah.1 * -1.0 * (1.0 - ratio));
                            }
                            if sum != 0.0 {
                                // WIP real collisions - very complex, I don't know enough physics rn but am learning
                                /*let m1 = self.objects[y].physics.mass;
                                let m2 = self.objects[x].physics.mass;
                                let total = m1 + m2;
                                let merged = self.objects[y].physics.velocity * m1 + self.objects[x].physics.velocity * m2;
                                self.objects[y].physics.velocity = (merged - ((self.objects[y].physics.velocity - self.objects[x].physics.velocity) * m2 * self.objects[y].physics.restitution)) / total;
                                self.objects[x].physics.velocity = (merged - ((self.objects[x].physics.velocity - self.objects[y].physics.velocity) * m1 * self.objects[x].physics.restitution)) / total;*/
                                // DUMB FULLY ELASTIC VERSION
                                //self.objects[y].physics.velocity = (self.objects[y].physics.velocity * (m1 - m2) + self.objects[x].physics.velocity * 2.0 * m1) / total;
                                //self.objects[x].physics.velocity = (self.objects[x].physics.velocity * (m2 - m1) + self.objects[y].physics.velocity * 2.0 * m2) / total;
                                // DUMB OLD VERSION
                                /*let (x_para, x_perp) = self.objects[x].physics.velocity.cut(intasectah.1);
                                let (y_para, y_perp) = self.objects[y].physics.velocity.cut(intasectah.1);
                                self.objects[y].physics.velocity = x_perp * (self.objects[y].physics.velocity.magnitude()/sum) + y_para; // add the old perpendicular component, allowing it to slide
                                self.objects[x].physics.velocity = y_perp * (self.objects[x].physics.velocity.magnitude()/sum) + x_para;*/
                                // VERY DUMB VERSION
                                let m1 = self.objects[y].exposed_properties.physics.mass;
                                let m2 = self.objects[x].exposed_properties.physics.mass;
                                let total = m1 + m2;
                                let (x_para, x_perp) = self.objects[x].exposed_properties.physics.velocity.cut(intasectah.1);
                                let (y_para, y_perp) = self.objects[y].exposed_properties.physics.velocity.cut(intasectah.1);
                                if !self.objects[y].exposed_properties.physics.fixed {
                                    self.objects[y].exposed_properties.physics.velocity = y_perp + x_para * (m2 / total);
                                }
                                if !self.objects[x].exposed_properties.physics.fixed {
                                    self.objects[x].exposed_properties.physics.velocity = x_perp + y_para * (m1 / total);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn send_physics_updates(&mut self) {
        let mut i : usize = 0;
        while i < self.objects.len() {
            let mut args_vec = vec![self.objects[i].get_id().to_string()];
            let phys = self.objects[i].get_physics_object();
            if phys.translated() || phys.rotated() || phys.resized() {
                args_vec.push(phys.cx().to_string());
                args_vec.push(phys.cy().to_string());
            }
            if phys.rotated() || phys.resized() {
                args_vec.push(phys.angle().to_string());
            }
            if phys.resized() {
                args_vec.push(phys.width().to_string());
                args_vec.push(phys.height().to_string());
            }
            if args_vec.len() > 1 {
                self.broadcast(ProtocolMessage {
                    command: 'M',
                    args: args_vec
                });
            }
            unsafe {
                (*(&mut self.objects as *mut Vec<GamePieceBase>))[i].update(self);
            }
            // Do death checks a bit late (pun not intended) so objects have a chance to self-rescue.
            if self.objects[i].dead() {
                unsafe {
                    (*(&mut self.objects as *mut Vec<GamePieceBase>))[i].die(self);
                }
                self.broadcast(ProtocolMessage {
                    command: 'd',
                    args: vec![self.objects[i].get_id().to_string()]
                });
                self.objects.remove(i);
                continue; // don't allow it to reach the increment
            }
            i += 1;
        }
    }

    fn delete_obj(&mut self, id : u32) {
        match self.obj_lookup(id) {
            Some (index) => {
                self.broadcast(ProtocolMessage {
                    command: 'd',
                    args: vec![id.to_string()]
                });
                self.objects.remove(index);
            },
            None => {} // No need to do anything, the object already doesn't exist
        }
    }

    fn clear_of_banner(&mut self, banner : usize) {
        if banner == 0 {
            return; // Never clear banner 0. That's just dumb. If you want to clear banner 0 manually delete the entire list.
        }
        let mut i : usize = 0;
        while i < self.objects.len() {
            let mut delted = false;
            if self.objects[i].get_banner() == banner {
                self.broadcast(ProtocolMessage {
                    command: 'd',
                    args: vec![self.objects[i].get_id().to_string()]
                });
                self.objects.remove(i);
                delted = true;
            }
            if !delted { // WASTING a SINGLE CYCLE. hee hee hee.
                i += 1;
            }
        }
    }

    fn mainloop(&mut self) {
        if self.mode == GameMode::Waiting {
            if self.is_io {
                self.start();
            }
            if self.autonomous.is_some() {
                if self.living_players >= self.autonomous.unwrap().0 {
                    let mut is_has_moreteam = true;
                    for team in &self.teams {
                        if team.live_count == self.living_players { // If one team holds all the players
                            is_has_moreteam = false;
                            break;
                        }
                    }
                    if is_has_moreteam {
                        self.autonomous.as_mut().unwrap().2 -= 1;
                        self.broadcast(ProtocolMessage {
                            command: '!',
                            args: vec![self.autonomous.unwrap().2.to_string()]
                        });
                        if self.autonomous.unwrap().2 <= 0 {
                            self.start();
                        }
                    }
                }
            }
        }
        else {
            if self.counter > 0 {
                self.counter -= 1;
            }
            else {
                self.flip();
            }
            if self.isnt_rtf == 0 {
                self.set_mode(GameMode::Play);
            }
            if !self.is_io {
                if self.living_players == 0 {
                    println!("GAME ENDS WITH A TIE");
                    self.broadcast(ProtocolMessage {
                        command: 'T',
                        args: vec![]
                    });
                    println!("Tie broadcast complete.");
                    self.reset();
                }
                else {
                    for team in &self.teams {
                        if team.live_count == self.living_players { // If one team holds all the players
                            println!("GAME ENDS WITH A WINNER");
                            self.broadcast(ProtocolMessage {
                                command: 'E',
                                args: vec![team.banner_id.to_string()]
                            });
                            self.reset();
                            return;
                        }
                    }
                    if self.living_players == 1 {
                        println!("GAME ENDS WITH A WINNER");
                        self.broadcast(ProtocolMessage {
                            command: 'E',
                            args: vec![self.winning_banner.to_string()]
                        });
                        self.reset();
                    }
                }
            }
            if self.mode == GameMode::Play {
                self.send_physics_updates();
            }
            self.broadcast_tx.send(ClientCommand::Tick (self.counter, (if self.mode == GameMode::Strategy { "1" } else { "0" }).to_string())).expect("Broadcast failed");
            if self.mode == GameMode::Play {
                self.deal_with_objects();
                self.place_timer -= 1;
                if self.place_timer <= 0 {
                    self.place_timer = rand::random::<u32>() % 200 + 50; // set to 2 for object count benchmarking
                    self.place_random_rubble();
                    println!("Object count: {}", self.objects.len());
                }
            }
        }
    }

    fn set_mode(&mut self, mode : GameMode) {
        self.counter = match mode {
            GameMode::Waiting => {
                if self.autonomous.is_some() {
                    self.autonomous.as_mut().unwrap().2 = self.autonomous.unwrap().3;
                }
                1.0
            },
            GameMode::Strategy => FPS * self.times.0,
            GameMode::Play => FPS * self.times.1
        } as u32;
        self.mode = mode;
    }

    fn flip(&mut self) {
        self.set_mode(match self.mode {
            GameMode::Strategy => GameMode::Play,
            GameMode::Play => GameMode::Strategy,
            GameMode::Waiting => GameMode::Waiting
        });
    }

    fn start(&mut self) {
        if self.mode == GameMode::Waiting {
            for _ in 0..((self.gamesize * self.gamesize) / 1000000.0) as u32 { // One per 1,000,000 square pixels, or 200, whichever is lower.
                self.place_random_rubble();
            }
            self.set_mode(GameMode::Strategy);
            println!("Game start.");
        }
        else {
            println!("That doesn't work here (not in waiting mode)");
        }
    }

    fn broadcast<'a>(&'a self, message : ProtocolMessage) {
        self.broadcast_tx.send(ClientCommand::Send(message)).expect("Broadcast failed");
    }

    /*fn broadcast_to_team<'a>(&'a self, message : ProtocolMessage, team : usize) {
        self.broadcast_tx.send(ClientCommand::SendToTeam(message, team)).expect("Broadcast failed");
    }*/

    fn chat(&self, content : String, sender : usize, priority : u8, to_whom : Option<usize>) {
        self.broadcast_tx.send(ClientCommand::ChatRoom (content, sender, priority, to_whom)).expect("Chat message failed");
    }

    fn add(&mut self, mut piece : GamePieceBase, banner : Option<usize>) -> u32 {
        piece.set_id(self.top_id);
        self.top_id += 1;
        if banner.is_some(){
            piece.set_banner(banner.unwrap());
            self.broadcast_tx.send(ClientCommand::AttachToBanner (piece.get_id(), banner.unwrap(), self.costs)).expect("Broadcast FAILED!");
        }
        self.broadcast(piece.get_new_message());
        let ret = piece.get_id();
        self.objects.push(piece);
        ret
    }

    fn authenticate(&self, password : String, spectator : bool) -> AuthState {
        if spectator {
            return AuthState::Spectator
        }
        if self.password == password || self.passwordless {
            return AuthState::Single;
        }
        if password == "" {
            return AuthState::Spectator;
        }
        else {
            for team in &self.teams {
                let is_allowed : bool = password == *team.password;
                if is_allowed {
                    return AuthState::Team (team.id, team.members.len() == 0);
                }
            }
        }
        return AuthState::Error;
    }

    fn banner_add(&mut self, mut dispatcha : Option<&mut Client>, mut banner : Arc<String>) -> usize {
        while self.banners.contains(&banner) {
            banner = Arc::new(banner.to_string() + ".copy");
        }
        let bannah = self.banners.len();
        let mut args = vec![bannah.to_string(), banner.to_string()];
        println!("Created new banner {}, {}", self.banners.len(), banner);
        if dispatcha.is_some() {
            dispatcha.as_mut().unwrap().banner = self.banners.len();
            println!("Added the banner to a client");
            if dispatcha.as_ref().unwrap().team.is_some() {
                args.push(self.teams[dispatcha.as_ref().unwrap().team.unwrap()].banner_id.to_string());
            }
        }
        self.banners.push(banner.clone());
        let message = ProtocolMessage {
            command: 'b',
            args
        };
        self.broadcast(message);
        bannah
    }

    fn get_team_of_banner(&self, banner : usize) -> Option<usize> {
        for team in 0..self.teams.len() {
            for member in &self.teams[team].members {
                if *member == banner {
                    return Some(team);
                }
            }
        }
        None
    }

    async fn metadata(&mut self, user : &mut Client) {
        println!("Sending metadata to {}", self.banners[user.banner]);
        for index in 0..self.banners.len() {
            let banner = &self.banners[index];
            let team = self.get_team_of_banner(index);
            println!("Team: {:?}", team);
            let mut args = vec![index.to_string(), banner.to_string()];
            if team.is_some(){
                args.push(self.teams[team.unwrap()].banner_id.to_string());
            }
            user.send_protocol_message(ProtocolMessage {
                command: 'b',
                args
            }).await;
        }
        for piece in &self.objects {
            user.send_protocol_message(piece.get_new_message()).await;
        }
        user.send_protocol_message(ProtocolMessage { // m also doubles as a "you have all the data" message
            command: 'm',
            args: vec![self.gamesize.to_string(), self.terrain_seed.to_string()]
        }).await;
    }

    async fn user_logged_in(&mut self, user : &mut Client) {
        self.authenticateds += 1;
        self.metadata(user).await;
    }

    async fn spectator_joined(&mut self, user : &mut Client) {
        self.metadata(user).await;
    }

    fn reset(&mut self) {
        println!("############## RESETTING ##############");
        /*while self.clients.len() > 0 {
            self.clients[0].lock().await.do_close = true;
            self.clients.remove(0);
        }*/
        self.broadcast_tx.send(ClientCommand::CloseAll).expect("Broadcast failed");
        while self.objects.len() > 0 {
            self.objects.remove(0);
        }
        self.set_mode(GameMode::Waiting);
        self.clear_banners();
        self.load_config();
    }

    fn clear_banners(&mut self) {
        println!("Clearing banners...");
        while self.banners.len() > 1 + self.teams.len() { // each team has a banner, lulz
            self.banners.remove(1); // Leave the first one, which is the null banner
        }
    }

    fn load_config(&mut self) {
        if self.config.is_some() {
            let config = self.config.as_ref().unwrap().clone();
            config.load_into(self);
        }
    }

    fn new_team(&mut self, name : String, password : String) {
        let banner = self.banner_add(None, Arc::new(name));
        let id = self.teams.len();
        self.teams.push(TeamData {
            id,
            banner_id: banner,
            password: Arc::new(password),
            members: vec![],
            live_count: 0
        });
    }
}


#[derive(Debug, Clone)]
pub struct ProtocolMessage {
    command : char,
    args    : Vec<String>
}


impl ProtocolMessage {
    /*fn singlet(command : char) -> ProtocolMessage {
        ProtocolMessage {
            command,
            args: vec![]
        }
    }*/

    fn parse_string(message : String) -> Option<Self> {
        let characters : Vec<char> = message.chars().collect();
        let command = characters[0];
        let mut args = vec![];
        let mut buffer : String = String::new();
        let mut i = 1;
        while i < characters.len() {
            let arg_end : u32 = i as u32 + characters[i] as u32;
            if arg_end >= characters.len() as u32{
                println!("[ WARNING ] Some idiot is trying to broadcast poison frames!");
                return None;
            }
            while i < arg_end as usize {
                i += 1;
                buffer.push(characters[i]);
            }
            i += 1;
            args.push(buffer.clone());
            buffer.clear();
        }
        Some(Self {
            command, args
        })
    }

    fn encode(&self) -> String {
        let mut r = String::new();
        r.push(self.command);
        for arg in &self.args {
            r.push(char::from_u32(arg.len() as u32).unwrap());
            r += &arg;
        }
        r
    }

    fn poison(&self, problem : &str) {
        println!("The client is poisoning us with {} ({})", self, problem);
    }
}


impl fmt::Display for ProtocolMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut thang = String::new();
        thang.push(self.command);
        thang += " with [";
        for arg in &self.args {
            thang += &arg;
            thang += ", ";
        }
        thang += "]";
        write!(f, "{}", thang)
    }
}


impl Client {
    fn new(tx : SplitSink<WebSocket, Message>, commandah : tokio::sync::mpsc::Sender<ServerCommand>) -> Self {
        Self {
            tx,
            is_authorized: false,
            score: 0,
            has_placed: false,
            banner: 0,
            m_castle: None,
            mode: ClientMode::None,
            team: None,
            commandah,
            is_team_leader: false,
            places_this_turn: 0,
            kys: false,
            a2a: 0
        }
    }

    async fn send_text(&mut self, text : &str) {
        match self.tx.send(Message::text(text)).await {
            Ok(_) => {}
            Err(_) => {
                println!("COULD NOT SEND A MESSAGE? This may be a borked pipe or summat'. Probably not fatal.");
            }
        }
    }

    async fn collect(&mut self, amount : i32) {
        self.score += amount;
        self.send_protocol_message(ProtocolMessage {
            command: 'S',
            args: vec![self.score.to_string()]
        }).await;
    }

    async fn send_protocol_message(&mut self, message : ProtocolMessage) {
        self.send_text(&(message.encode())).await;
    }

    async fn send_error(&mut self, error : u32) {
        self.send_protocol_message(ProtocolMessage {
            command: 'e',
            args: vec![error.to_string()]
        }).await;
    }

    async fn send_warning(&mut self, warning : u32) {
        self.send_protocol_message(ProtocolMessage {
            command: 'w',
            args: vec![warning.to_string()]
        }).await;
    }

    async fn send_singlet(&mut self, thing : char) {
        self.send_protocol_message(ProtocolMessage {
            command: thing,
            args: vec![]
        }).await;
    }

    async fn retaliate_from_poison(&mut self) {
        self.send_protocol_message(ProtocolMessage {
            command: '-',
            args: vec!["Your poisoning attempts have FAILED. EAT DISCONNECT!".to_string()]
        }).await;
        self.kys = true;
        println!("Retaliating");
    }

    async fn handle(&mut self, message : ProtocolMessage, mut server : tokio::sync::MutexGuard<'_, Server>) {
        if message.command == 'c' && !self.is_authorized {
            if server.new_user_can_join() {
                let lockah = server.authenticate(message.args[0].clone(), message.args[2] == "spectator");
                match lockah { // If you condense this, for === RUST REASONS === it keeps the mutex locked.
                    AuthState::Error => {
                        println!("New user has invalid password!");
                        self.send_error(0).await;
                        return;
                    },
                    AuthState::Single => {
                        println!("New user has authenticated as single player");
                        self.send_singlet('s').await;
                        server.user_logged_in(self).await;
                        server.banner_add(Some(self), Arc::new(message.args[1].clone()));
                        self.is_authorized = true;
                    },
                    AuthState::Team (team, tl) => {
                        println!("New user has authenticated as player in team {}", server.banners[server.teams[team].banner_id]);
                        self.team = Some(team);
                        self.send_singlet('s').await;
                        if tl {
                            self.send_singlet('?').await;
                            self.is_team_leader = true;
                        }
                        server.user_logged_in(self).await;
                        server.banner_add(Some(self), Arc::new(message.args[1].clone()));
                        server.teams[team].members.push(self.banner);
                        self.is_authorized = true;
                    },
                    AuthState::Spectator => {
                        println!("Spectator joined!");
                        self.send_warning(0).await;
                        server.spectator_joined(self).await;
                    }
                }
                self.mode = match message.args[2].as_str() {
                    "normal" => ClientMode::Normal,
                    "defender" => ClientMode::Defense,
                    "rtf" => ClientMode::RealTimeFighter,
                    _ => ClientMode::Normal
                };
            }
            else {
                println!("New user can't join!");
                self.send_warning(0).await;
                server.spectator_joined(self).await;
            }
        }
        else if self.is_authorized {
            match message.command {
                'p' => {
                    if message.args.len() == 3 {
                        if self.places_this_turn >= 30 {
                            return;
                        }
                        if server.mode != GameMode::Strategy && message.args[0] != "c"{ // if it is trying to place an object, but it isn't strat mode and it isn't placing a castle
                            message.poison("CLIENT IS ATTEMPTING TO PLACE AN OBJECT OUTSIDE OF STRATEGY MODE");
                            // originally, this retaliated, but now it just refuses. the retaliation was a problem.
                            return; // can't place things if it ain't strategy. Also this is poison.
                        }
                        self.places_this_turn += 1;
                        let tp = &message.args[0];
                        let x = message.args[1].parse::<f32>();
                        let y = message.args[2].parse::<f32>();
                        if x.is_ok() && y.is_ok(){
                            let x = x.unwrap();
                            let y = y.unwrap();
                            if x < 0.0 || x > server.gamesize || y < 0.0 || y > server.gamesize { 
                                self.kys = true;
                                return;
                            }
                            match tp.as_str() {
                                "c" => {
                                    if server.mode != GameMode::Waiting && !server.is_io { // you can't place castles if it's waiting and not an io game
                                        return; // can't place castles if it isn't in WAITING.
                                    }
                                    if !self.has_placed {
                                        self.has_placed = true;
                                        server.costs = false;
                                        self.m_castle = Some(server.place_castle(x, y, self.mode == ClientMode::RealTimeFighter, Some(self.banner)));
                                        self.commandah.send(ServerCommand::LivePlayerInc (self.team, self.mode)).await.expect("Broadcast failed");
                                        match self.mode {
                                            ClientMode::Normal => {
                                                server.place_basic_fighter(x - 200.0, y, PI, Some(self.banner));
                                                server.place_basic_fighter(x + 200.0, y, 0.0, Some(self.banner));
                                                server.place_basic_fighter(x, y - 200.0, 0.0, Some(self.banner));
                                                server.place_basic_fighter(x, y + 200.0, 0.0, Some(self.banner));
                                                self.collect(100).await;
                                            },
                                            ClientMode::RealTimeFighter => {
                                                server.place_basic_fighter(x - 100.0, y, PI, Some(self.banner));
                                                server.place_basic_fighter(x + 100.0, y, 0.0, Some(self.banner));
                                                self.send_singlet('A').await;
                                                self.a2a += 1;
                                            },
                                            ClientMode::Defense => {
                                                server.place_basic_fighter(x - 200.0, y, PI, Some(self.banner));
                                                server.place_basic_fighter(x + 200.0, y, 0.0, Some(self.banner));
                                                server.place_turret(x, y - 200.0, 0.0, Some(self.banner));
                                                server.place_turret(x, y + 200.0, 0.0, Some(self.banner));
                                                self.collect(25).await;
                                            },
                                            _ => {

                                            }
                                        }
                                        server.costs = true;
                                    }
                                    else {
                                        message.poison("MULTIPLE CASTLE PLACE ATTEMPTS");
                                        self.retaliate_from_poison().await;
                                    }
                                },
                                "f" => {
                                    server.place_basic_fighter(x, y, 0.0, Some(self.banner));
                                },
                                "w" => {
                                    server.place_wall(x, y, Some(self.banner));
                                },
                                "t" => {
                                    server.place_tie_fighter(x, y, 0.0, Some(self.banner));
                                },
                                "s" => {
                                    server.place_sniper(x, y, 0.0, Some(self.banner));
                                },
                                "h" => {
                                    server.place_missile(x, y, 0.0, Some(self.banner));
                                },
                                "T" => {
                                    server.place_turret(x, y, 0.0, Some(self.banner));
                                },
                                "n" => {
                                    server.place_nuke(x, y, 0.0, Some(self.banner));
                                },
                                "F" => {
                                    match self.m_castle {
                                        Some(cid) => {
                                            match server.obj_lookup(cid) {
                                                Some(index) => {
                                                    let fort = server.place_fort(x, y, 0.0, Some(self.banner));
                                                    server.objects[index].add_fort(fort);
                                                }
                                                None => {}
                                            }
                                        }
                                        None => {}
                                    }
                                },
                                "m" => {
                                    server.place_mls(x, y, 0.0, Some(self.banner));
                                },
                                "a" => {
                                    server.place_antirtf_missile(x, y, 0.0, Some(self.banner));
                                },
                                "K" => {
                                    server.place_carrier(x, y, 0.0, Some(self.banner));
                                },
                                &_ => {
                                    message.poison("INVALID PLACE TYPE");
                                }
                            };
                        }
                        else {
                            message.poison("INVALID INTEGERS");
                            self.retaliate_from_poison().await;
                        }
                    }
                    else {
                        message.poison("INVALID ARGUMENT LENGTH");
                    }
                },
                'C' => {
                    let mut amount = match message.args[0].parse::<i32>() {
                        Ok(numbah) => numbah,
                        Err(_) => {
                            message.poison("INVALID INTEGERS");
                            self.retaliate_from_poison().await;
                            0
                        }
                    };
                    amount = amount.abs();
                    println!("Costing an amount of money equal to {amount} coins");
                    self.collect(-amount).await;
                },
                'm' => {
                    let id = match message.args[0].parse::<u32>() {
                        Ok(numbah) => numbah,
                        Err(_) => {
                            message.poison("INVALID INTEGERS");
                            self.retaliate_from_poison().await;
                            return;
                        }
                    };
                    let x = match message.args[1].parse::<f32>() {
                        Ok(numbah) => numbah,
                        Err(_) => {
                            message.poison("INVALID INTEGERS");
                            self.retaliate_from_poison().await;
                            return;
                        }
                    };
                    let y = match message.args[2].parse::<f32>() {
                        Ok(numbah) => numbah,
                        Err(_) => {
                            message.poison("INVALID INTEGERS");
                            self.retaliate_from_poison().await;
                            return;
                        }
                    };
                    let a = match message.args[3].parse::<f32>() {
                        Ok(numbah) => numbah,
                        Err(_) => {
                            message.poison("INVALID INTEGERS");
                            self.retaliate_from_poison().await;
                            return;
                        }
                    };
                    for object in &mut server.objects {
                        if object.get_id() == id && object.get_banner() == self.banner {
                            object.exposed_properties.goal_x = x;
                            object.exposed_properties.goal_y = y;
                            object.exposed_properties.goal_a = a;
                        }
                    }
                },
                'A' => { // AIR TO AIR!
                    if self.a2a == 0 {
                        self.kys = true;
                    }
                    if self.m_castle.is_some() {
                        match message.args[0].parse::<u32>() {
                            Ok(numbah) => {
                                let mut obj_vec = None;
                                for obj in &server.objects {
                                    if obj.get_id() == numbah {
                                        obj_vec = Some(obj.exposed_properties.physics.vector_position());
                                    }
                                }
                                match obj_vec {
                                    None => {},
                                    Some (vec) => {
                                        match self.m_castle {
                                            Some(castleid) => {
                                                match server.obj_lookup(castleid) {
                                                    Some (index) => {
                                                        if (server.objects[index].exposed_properties.physics.vector_position() - vec).magnitude() < 1500.0 {
                                                            self.a2a -= 1;
                                                            let pos = server.objects[index].exposed_properties.physics.vector_position() + Vector2::new_from_manda(50.0, server.objects[index].exposed_properties.physics.angle());
                                                            let launchangle = server.objects[index].exposed_properties.physics.angle() - PI/2.0; // rust requires this to be explicit because of the dumbass borrow checker
                                                            server.place_air2air(pos.x, pos.y, launchangle, numbah, Some(self.banner));
                                                        }
                                                    }
                                                    None => {}
                                                }
                                            }
                                            None => {}
                                        }
                                    }
                                }
                            },
                            Err(_) => {
                                message.poison("INVALID INTEGERS");
                                self.retaliate_from_poison().await;
                                return;
                            }
                        };
                    }
                },
                'R' => {
                    if server.mode == GameMode::Play && self.m_castle.is_some(){
                        match self.m_castle {
                            Some (cid) => {
                                match server.obj_lookup(cid) {
                                    Some (index) => {
                                        let mut thrust = 0.0;
                                        let mut resistance = 1.0;
                                        let mut angle_thrust = 0.0;
                                        if message.args[0] == "1" { // THRUST
                                            thrust = 2.0;
                                        }
                                        if message.args[1] == "1" { // TURN LEFT
                                            angle_thrust -= 0.02;
                                        }
                                        if message.args[2] == "1" { // TURN RIGHT
                                            angle_thrust += 0.02;
                                        }
                                        if message.args[3] == "1" { // AIRBRAKE
                                            resistance = 0.8;
                                        }
                                        server.objects[index].exposed_properties.shooter_properties.suppress = !(message.args[4] == "1");
                                        let thrust = Vector2::new_from_manda(thrust, server.objects[index].exposed_properties.physics.angle() - PI/2.0);
                                        server.objects[index].exposed_properties.physics.velocity += thrust;
                                        server.objects[index].exposed_properties.physics.velocity *= resistance;
                                        server.objects[index].exposed_properties.physics.angle_v += angle_thrust;
                                        server.objects[index].exposed_properties.physics.angle_v *= 0.9;
                                        server.objects[index].exposed_properties.physics.angle_v *= resistance;
                                    }
                                    None => {}
                                }
                            }
                            None => {}
                        }
                    }
                },
                'T' => { // Talk
                    /*let args = vec![format!("<span style='color: pink;'>{} SAYS</span>: {}", server.banners[self.banner], message.args[0])];
                    if self.team.is_some() && message.args[0].chars().nth(0) != Some('!') {
                        server.broadcast_to_team(ProtocolMessage {
                            command: 'B',
                            args
                        }, self.team.unwrap());
                    }
                    else {
                        server.broadcast(ProtocolMessage {
                            command: 'B',
                            args
                        });
                    }*/
                    server.chat(message.args[0].clone(), self.banner, if self.is_team_leader { 1 } else { 0 },
                        if self.team.is_none() || message.args[1] == "broadcast" {
                            None
                        }
                        else {
                            self.team
                        }
                    );
                    println!("{} says {}", server.banners[self.banner], message.args[0]);
                },
                'U' => {
                    // Upgrade.
                    if message.args.len() != 2 {
                        message.poison("INVALID ARGUMENT LENGTH");
                    }
                    let id = match message.args[0].parse::<u32>() {
                        Ok(id) => id,
                        Err(_) => {
                            message.poison("INVALID ID");
                            return;
                        }
                    };
                    println!("Upgrading id {} to {}", id, message.args[1]);
                    let upg = message.args[1].clone();
                    let price = *server.upg_costs.entry(message.args[1].clone()).or_insert(0);
                    if self.score >= price {
                        self.collect(-price).await;
                        for object in &mut server.objects {
                            if object.get_id() == id {
                                object.upgrade(upg);
                                break;
                            }
                        }
                        server.broadcast(ProtocolMessage {
                            command: 'u',
                            args: vec![id.to_string(), message.args[1].clone()]
                        });
                    }
                    else { // something nefarious is going on!
                        self.kys = true;
                    }
                }
                _ => {
                    message.poison("INAPPROPRIATE COMMAND");
                }
            }
        }
        else {
            message.poison("INVALID COMMAND");
        }
    }

    fn close(&self) {
        println!("Client close routine!");
    }

    fn is_alive(&self, server : tokio::sync::MutexGuard<'_, Server>) -> bool {
        if self.m_castle.is_some() {
            if server.obj_lookup(self.m_castle.unwrap()).is_some() {
                return true;
            }
        }
        false
    }
}


async fn got_client(websocket : WebSocket, server : Arc<Mutex<Server>>, broadcaster : tokio::sync::broadcast::Sender<ClientCommand>, commandset : tokio::sync::mpsc::Sender<ServerCommand>){
    commandset.send(ServerCommand::Connect).await.unwrap();
    let mut receiver = broadcaster.subscribe();
    let (tx, mut rx) = websocket.split();
    let mut moi = Client::new(tx, commandset);/*
    if server.lock().await.passwordless {
        moi.send_singlet('p').await;
    }*/
    // passwordless broadcasts aren't really relevant any more
    'cliloop: loop {
        select! {
            insult = rx.next().fuse() => {
                match insult {
                    Some(result) => {
                        let msg = match result {
                            Ok(msg) => msg,
                            Err(e) => {
                                println!("UH NOES! A SOCKET BORKY HAPPENDY! POOPLES! {e}");
                                println!("This is not critical, your server *should* survive.");
                                break 'cliloop;
                            }
                        };
                        if msg.is_text(){
                            let text = match msg.to_str() {
                                Ok(text) => text,
                                Err(()) => ""
                            };
                            if text == "_"{
                                moi.send_text("_").await;
                            }
                            else if text == "HELLO MMOSG" {
                                moi.send_text("HELLO MMOSG").await;
                            }
                            else {
                                let p = ProtocolMessage::parse_string(text.to_string());
                                if p.is_some() {
                                    let serverlock = server.lock().await;
                                    moi.handle(p.unwrap() /* If it made it this far, there's data to unwrap */, serverlock).await;
                                    if moi.kys { // if it's decided to break the connection
                                        break 'cliloop;
                                    }
                                }
                                else {
                                    break 'cliloop; // don't take no truck with poison frames no more
                                }
                            }
                        }
                    },
                    None => {
                        println!("Client gracefully disconnected");
                        break 'cliloop;
                    }
                }
            },
            command = receiver.recv().fuse() => {
                match command {
                    Ok (ClientCommand::Tick (counter, modechar)) => {
                        if modechar == "0" { // if it's play mode
                            moi.places_this_turn = 0;
                        }
                        let mut schlock = server.lock().await;
                        schlock.winning_banner = moi.banner;
                        let mut args = vec![counter.to_string(), modechar];
                        if moi.m_castle.is_some() {
                            match schlock.obj_lookup(moi.m_castle.unwrap()) {
                                Some (index) => {
                                    args.push(schlock.objects[index].health().to_string());
                                },
                                None => {}
                            }
                        }
                        moi.send_protocol_message(ProtocolMessage {
                            command: 't',
                            args
                        }).await;
                        if !moi.is_alive(schlock) {
                            if moi.m_castle.is_some() {
                                moi.m_castle = None;
                                moi.send_singlet('l').await;
                                moi.commandah.send(ServerCommand::LivePlayerDec (moi.team, moi.mode)).await.expect("Broadcast failed");
                            }
                        }
                    },
                    Ok (ClientCommand::Send (message)) => {
                        moi.send_protocol_message(message).await;
                    },
                    /*Ok (ClientCommand::SendToTeam (message, team)) => {
                        if moi.team.is_some() && moi.team.unwrap() == team {
                            moi.send_protocol_message(message).await;
                        }
                    },*/
                    Ok (ClientCommand::ScoreTo (banner, amount)) => {
                        if moi.banner == banner {
                            moi.collect(amount).await;
                            /*moi.score += amount;
                            moi.send_protocol_message(ProtocolMessage {
                                command: 'S',
                                args: vec![moi.score.to_string()]
                            }).await;*/
                        }
                    },
                    Ok (ClientCommand::CloseAll) => {
                        break 'cliloop;
                    },
                    Ok (ClientCommand::ChatRoom (content, sender, priority, target)) => {
                        if target.is_none() || target == moi.team {
                            moi.send_protocol_message(ProtocolMessage {
                                command: 'B',
                                args: vec![content, sender.to_string(), priority.to_string()]
                            }).await;
                        }
                    },
                    Ok (ClientCommand::GrantA2A (to)) => {
                        if to == moi.banner && moi.mode == ClientMode::RealTimeFighter {
                            moi.a2a += 1;
                            moi.send_singlet('A').await;
                        }
                    },
                    Ok (ClientCommand::AttachToBanner (id, banner, does_cost)) => {
                        if banner == moi.banner {
                            let mut reject : bool = false;
                            if does_cost {
                                let lock = server.lock().await;
                                match lock.obj_lookup(id) {
                                    Some (index) => {
                                        if moi.score >= lock.objects[index].cost() {
                                            moi.collect(-lock.objects[index].cost()).await;
                                        }
                                        else {
                                            reject = true;
                                        }
                                    }
                                    None => {
                                        reject = true;
                                    }
                                }
                            }
                            if reject {
                                println!("REJECTING OBJECT WE CAN'T AFFORD!");
                                moi.commandah.send(ServerCommand::RejectObject (id)).await.expect("Failed!");
                                break 'cliloop; // something nefarious happened; let's disconnect
                            }
                            else {
                                moi.send_protocol_message(ProtocolMessage {
                                    command: 'a',
                                    args: vec![id.to_string()]
                                }).await;
                            }
                        }
                    }
                    //_ => {}
                    Err (_) => {

                    }
                }
            }
        }
    }
    let mut serverlock = server.lock().await;
    if moi.m_castle.is_some() && serverlock.obj_lookup(moi.m_castle.unwrap()).is_some() {
        moi.commandah.send(ServerCommand::LivePlayerDec (moi.team, moi.mode)).await.expect("Broadcast failed");
    }
    moi.close();
    if moi.team.is_some(){ // Remove us from the team
        let mut i = 0;
        while i < serverlock.teams[moi.team.unwrap()].members.len() {
            if serverlock.teams[moi.team.unwrap()].members[i] == moi.banner {
                serverlock.teams[moi.team.unwrap()].members.remove(i);
                break;
            }
            i += 1;
        }
    }
    /*if !morlock.do_close { // If it's not been force-closed by the server (which handles closing if force-close happens)
        let index = serverlock.clients.iter().position(|x| Arc::ptr_eq(x, &moi)).unwrap();
        serverlock.clients.remove(index);
    }*/
    if moi.is_authorized {
        serverlock.authenticateds -= 1;
    }
    if serverlock.is_io || serverlock.mode == GameMode::Waiting {
        serverlock.clear_of_banner(moi.banner);
    }
    moi.commandah.send(ServerCommand::Disconnect).await.unwrap();
    println!("Dropped client");
}

//
//
//============[]             \       /   
//============[]  0      0   _|     |_
//============[]--0------0--/ _   _   \
//============[]------0-----|   _   _ |
//============[]      0      \_______/
//============[]
//
//
// YOU'VE BEEN PWNED by a VENUS FIRETRAP LV. 3
// Your spells: Extinguish lv. 1, air blast lv. 2
// HP: -1 of 8; XP: 80; LV: 2;
// YOU ARE DEAD! INSERT A COIN TO CONTINUE!

fn input(prompt: &str) -> String {
    use std::io;
    use std::io::{BufRead, Write};
    print!("{}", prompt);
    io::stdout().flush().expect("Input failed!");
    io::stdin()
        .lock()
        .lines()
        .next()
        .unwrap()
        .map(|x| x.trim_end().to_owned()).expect("Input failed!")
}


#[derive(Debug)]
enum ServerCommand {
    Start,
    Flip,
    IoModeToggle,
    PasswordlessToggle,
    Autonomous (u32, u32, u32),
    TeamNew (String, String),
    LivePlayerInc (Option<usize>, ClientMode),
    LivePlayerDec (Option<usize>, ClientMode),
    Connect,
    Disconnect,
    Broadcast (String),
    RejectObject (u32)
}


#[tokio::main]
async fn main(){
    let args: Vec<String> = std::env::args().collect();
    use tokio::sync::mpsc::error::TryRecvError;
    let mut rng = rand::thread_rng();
    let (broadcast_tx, _rx) = tokio::sync::broadcast::channel(128); // Give _rx a name because we want it to live to the end of this function; if it doesn't, the tx will be invalidated. or something.
    let mut server = Server {
        mode                : GameMode::Waiting,
        password            : "".to_string(),
        config              : Some(Arc::new(Config::new(&args[1]))),
        objects             : vec![],
        teams               : vec![],
        gamesize            : 5000.0,
        authenticateds      : 0,
        terrain_seed        : rng.gen(),
        banners             : vec![Arc::new("None".to_string())],
        top_id              : 1, // id 0 is the "none" id
        counter             : 1,
        costs               : true,
        place_timer         : 100,
        autonomous          : None,
        is_io               : false,
        passwordless        : true,
        broadcast_tx        : broadcast_tx.clone(),
        living_players      : 0,
        winning_banner      : 0,
        isnt_rtf            : 0,
        times               : (40.0, 20.0),
        clients_connected   : 0,
        is_headless         : false,
        permit_npcs         : true,
        port                : 0,
        sql                 : "default.db".to_string(),
        upg_costs           : HashMap::from([
            ("b".to_string(), 30),
            ("f".to_string(), 70),
            ("h".to_string(), 150),
            ("s".to_string(), 40),
        ])
    };
    //rx.close().await;
    server.load_config();
    let port = server.port;
    let headless = server.is_headless;
    println!("Started server with password {}, terrain seed {}", server.password, server.terrain_seed);
    let server_mutex = Arc::new(Mutex::new(server));
    let server_mutex_loopah = server_mutex.clone();
    let (commandset, mut commandget) = tokio::sync::mpsc::channel(32); // fancy number
    let commandset_clone = commandset.clone();
    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis((1000.0/FPS) as u64));
        let connection = sqlite::open(server_mutex_loopah.lock().await.sql.clone()).unwrap();
        let init_query = "CREATE TABLE IF NOT EXISTS logins (banner TEXT, password TEXT, highscore INTEGER, wins INTEGER, losses INTEGER);CREATE TABLE IF NOT EXISTS teams_records (teamname TEXT, wins INTEGER, losses INTEGER);";
        connection.execute(init_query).unwrap();
        loop {
            //use tokio::time::Instant;
            interval.tick().await;
            let mut lawk = server_mutex_loopah.lock().await;
            //let start = Instant::now();
            lawk.mainloop();
            //println!("Mainloop took {:?}", start.elapsed());
            match commandget.try_recv() {
                Ok (ServerCommand::Start) => {
                    lawk.start();
                },
                Ok (ServerCommand::RejectObject (id)) => {
                    lawk.delete_obj(id);
                },
                Ok (ServerCommand::Flip) => {
                    lawk.flip();
                },
                Ok (ServerCommand::TeamNew (name, password)) => {
                    lawk.new_team(name, password);
                },
                Ok (ServerCommand::Autonomous (min_players, max_players, auto_timeout)) => {
                    lawk.autonomous = Some((min_players, max_players, auto_timeout, auto_timeout));
                },
                Ok (ServerCommand::IoModeToggle) => {
                    lawk.is_io = !lawk.is_io;
                    println!("Set io mode to {}", lawk.is_io);
                },
                Ok (ServerCommand::Disconnect) => {
                    lawk.clients_connected -= 1;
                    if lawk.clients_connected == 0 {
                        lawk.clear_banners();
                    }
                },
                Ok (ServerCommand::Connect) => {
                    lawk.clients_connected += 1;
                },
                Ok (ServerCommand::Broadcast (message)) => {
                    lawk.chat(message, 0, 6, None);
                },
                Ok (ServerCommand::PasswordlessToggle) => {
                    lawk.passwordless = !lawk.passwordless;
                    lawk.broadcast(ProtocolMessage {
                        command: 'p',
                        args: vec![]
                    });
                    println!("Set passwordless mode to {}", lawk.passwordless);
                },
                Ok(ServerCommand::LivePlayerInc (team, mode)) => {
                    if mode != ClientMode::RealTimeFighter {
                        println!("{:?} isn't an rtf", mode);
                        lawk.isnt_rtf += 1;
                    }
                    lawk.living_players += 1;
                    if team.is_some() {
                        lawk.teams[team.unwrap()].live_count += 1;
                    }
                    println!("New live player. Living players: {}", lawk.living_players);
                },
                Ok(ServerCommand::LivePlayerDec (team, mode)) => {
                    if mode != ClientMode::RealTimeFighter {
                        lawk.isnt_rtf -= 1;
                    }
                    lawk.living_players -= 1;
                    if team.is_some() {
                        lawk.teams[team.unwrap()].live_count -= 1;
                    }
                    println!("Player died. Living players: {}", lawk.living_players);
                },
                Err (TryRecvError::Disconnected) => {
                    println!("The channel handling server control was disconnected!");
                },
                Err (TryRecvError::Empty) => {} // Do nothing; we expect it to be empty quite often.
            }
        }
    });

    if !headless {
        tokio::task::spawn(async move {
            loop {
                let command = input("");
                let to_send = match command.as_str() {
                    "start" => { // Notes: starting causes the deadlock, but flipping doesn't, so the problem isn't merely locking/unlocking.
                        ServerCommand::Start
                    },
                    "flip" => {
                        println!("Flipping stage");
                        ServerCommand::Flip
                    },
                    "team new" => {
                        let name = input("Team name: ");
                        let password = input("Team password: ");
                        ServerCommand::TeamNew(name, password)
                    },
                    "toggle iomode" => {
                        ServerCommand::IoModeToggle
                    },
                    "toggle passwordless" => {
                        ServerCommand::PasswordlessToggle
                    },
                    "broadcast" => {
                        ServerCommand::Broadcast (input("Message: "))
                    },
                    "autonomous" => {
                        let min_players = match input("Minimum player count to start: ").parse::<u32>() {
                            Ok(num) => num,
                            Err(_) => {
                                println!("Invalid number.");
                                continue;
                            }
                        };
                        let max_players = match input("Maximum player count: ").parse::<u32>() {
                            Ok(num) => num,
                            Err(_) => {
                                println!("Invalid number.");
                                continue;
                            }
                        };
                        let auto_timeout = match input("Timer: ").parse::<u32>() {
                            Ok(num) => num,
                            Err(_) => {
                                println!("Invalid number.");
                                continue;
                            }
                        };
                        ServerCommand::Autonomous (min_players, max_players, auto_timeout)
                    },
                    _ => {
                        println!("Invalid command.");
                        continue;
                    }
                };
                commandset.send(to_send).await.expect("OOOOOOPS");
            }
        });
    }

    let servah = warp::any().map(move || server_mutex.clone());
    let websocket = warp::path("game")
        .and(warp::ws())
        .and(servah)
        .and(warp::any().map(move || broadcast_tx.clone()))
        .and(warp::any().map(move || commandset_clone.clone())) // dumbest line in the history of rust
        .map(|ws : warp::ws::Ws, servah, sendah, commandset| {
            ws.on_upgrade(move |websocket| got_client(websocket, servah, sendah, commandset))
        });
    let stat = warp::any()
        .and(warp::fs::dir("../"));
    
    let routes = stat.or(websocket);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}


#[cfg(test)]
pub mod tests {
    use crate::Vector2;
    use crate::gamepiece::npc;
    use crate::BoxShape;
    use crate::functions::*;
    use std::f32::consts::PI;
    use crate::leaderboard;
    #[test]
    fn check_vector_creation() {
        let vec = Vector2::new_from_manda(1.0, 0.0);
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 0.0);
        let vec = Vector2::new(1.0, 0.0);
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 0.0);
        let vec = Vector2::empty();
        assert!(vec.is_zero());
    }

    #[test]
    fn check_vector_addition() {
        let vec1 = Vector2::new(1.0, 0.0);
        let vec2 = Vector2::new(-1.0, 0.0);
        assert_eq!(vec1 + vec2, Vector2::empty());
        let vec3 = Vector2::new(1.0, 1.0);
        assert_eq!(vec1 + vec3, Vector2::new(2.0, 1.0));
    }

    #[test]
    fn check_vector_cutting() {
        let axis = Vector2::new(1.0, 1.0);
        let vector = Vector2::new(-1.0, 1.0);
        let (para, perp) = vector.cut(axis);
        assert!(para.is_basically(0.0));
        assert!(perp.is_basically(2.0_f32.sqrt()));
        let axis2 = Vector2::new(0.0, 1.0);
        let vector = Vector2::new(-1.0, -1.0);
        let (para, perp) = vector.cut(axis2);
        println!("Yep {}", para.magnitude());
        assert!(para.is_basically(1.0));
        assert!(perp.is_basically(1.0));
    }

    #[test]
    fn check_loopize_basics() {
        assert_eq!(loopize(1.0, 2.0), -1.0);
        assert_eq!(loopize(1.0, 0.0), 1.0);
    }

    #[test]
    fn check_loopize_complex() {
        assert_eq!(loopize(1.0, -1.0), 2.0);
        assert_eq!(loopize(-1.0, 1.0), -2.0);
        assert_eq!(loopize_about(2.0, 0.0, 3.0), -1.0);
    }

    #[test]
    fn check_box_contains() {
        let mut shape = BoxShape {
            x : 0.0,
            y : 0.0,
            w : 10.0,
            h : 10.0,
            a : 0.0
        };
        assert!(shape.contains(Vector2::new(-4.0, 0.0)));
        assert!(!shape.contains(Vector2::new(-5.0, 0.0)));
        shape.a = PI/4.0;
        assert!(shape.contains(Vector2::new(-7.0, 0.0)));
        assert!(!shape.contains(Vector2::new(-8.0, 0.0)));
        shape.a = PI/8.0;
        assert!(shape.contains(Vector2::new(-4.0, 0.0)));
    }

    #[test]
    fn leaderboard_read() {
        leaderboard::read_leaderboard("fancy_world_io.leaderboard");
    }
}