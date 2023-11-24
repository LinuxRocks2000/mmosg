/*
    By Tyler Clarke
*/

#![allow(non_camel_case_types)]
pub mod vector;
pub mod physics;
pub mod gamepiece;
pub mod config;
pub mod functions;
use crate::vector::Vector2;
use std::vec::Vec;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::gamepiece::*;
use std::f32::consts::PI;
use rand::Rng;
use crate::gamepiece::fighters::*;
use crate::gamepiece::misc::*;
use crate::gamepiece::npc;
use crate::physics::BoxShape;
use crate::config::Config;
use futures::future::FutureExt; // for `.fuse()`
use tokio::select;
use crate::gamepiece::BulletType;
use protocol_v3::server::{WebSocketServer, WebSocketClientStream};
use protocol_v3::protocol::ProtocolFrame;
use protocol_v3::protocol_v3_macro::ProtocolFrame;

const FPS : f32 = 30.0;


#[derive(PartialEq, Copy, Clone, Debug)]
pub enum ClientMode {
    None,
    Normal,
    Defense,
    RealTimeFighter
}


pub struct Client {
    socket            : WebSocketClientStream,
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
    a2a               : u16,
    walls_remaining   : u16,
    walls_cap         : u16,
    game_cmode        : GameMode
}


#[derive(ProtocolFrame, Debug, Clone)]
pub enum ServerToClient {
    Pong,
    Tick (u32, u8), // counter, mode.
    HealthUpdate (u32, f32), // update the health for any object.
    SetPasswordless (bool), 
    BannerAdd (u32, String), // banner id, banner text
    BannerAddToTeam (u32, u32), // banner we're adding to a team, team banner.
    End (u32), // banner that won: this can be a team.
    Metadata (f32, u32), 
    SetScore (i32),
    Chat (String, u32, u8), // message, sender, priority
    BadPassword,
    Welcome,
    YouAreSpectating,
    YouAreTeamLeader,
    A2A (u16), // send the client the how many A2A weapons it has
    UpgradeThing (u32, String),
    YouLose,
    Add (u32),
    Radiate (u32, f32),
    New (u32, u8, f32, f32, f32, bool, u32, f32, f32), // id, type, x, y, a, editable, banner, w, h
    MoveObjectFull (u32, f32, f32, f32, f32, f32), // id, x, y, a, w, h. inefficient (25 bytes altogether); use a smaller one like MoveObjectXY or MoveObjectA or MoveObjectXYA if possible.
    Delete (u32),
    Tie
}

#[derive(ProtocolFrame, Debug, Clone)]
pub enum ClientToServer {
    Ping,
    Connect (String, String, String), // password, banner, mode
    Place (f32, f32, u8),
    Cost (i32),
    Move (u32, f32, f32, f32),
    LaunchA2A (u32),
    PilotRTF (bool, bool, bool, bool, bool),
    Chat (String, bool), // the bool is if it's sent to everyone or not
    UpgradeThing (u32, String),
    SelfTest (bool, u8, u16, u32, i32, f32, String),
    Shop (u8)
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
    Send (ServerToClient),
    Tick (u32, GameMode),
    ScoreTo (usize, i32),
    CloseAll,
    ChatRoom (String, usize, u8, Option<usize>), // message, sender, priority
    GrantA2A (usize),
    AttachToBanner (u32, usize, bool),
    SetCastle (usize, u32) // banner to set, id of the castle
}


pub struct Server {
    mode              : GameMode,
    password          : String,
    objects           : Vec<GamePieceBase>,
    teams             : Vec<TeamData>,
    banners           : Vec<String>,
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
    worldzone_count   : usize,
    zones             : Vec<Vec<usize>>,
    vvlm              : bool
}

#[derive(Debug)]
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

    fn upgrade_thing_to(&mut self, thing : u32, upgrade : String) {
        for object in &mut self.objects {
            if object.get_id() == thing {
                object.upgrade(upgrade.clone());
                self.broadcast(ServerToClient::UpgradeThing (thing, upgrade));
                break;
            }
        }
    }

    fn upgrade_next_tier(&mut self, thing : u32, upgrade : String) {
        for object in &mut self.objects {
            if object.get_id() == thing {
                let mut stack = 1;
                let mut stackstring = stack.to_string();
                while object.upgrades.contains(&(upgrade.clone() + if stack == 1 { "" } else { stackstring.as_str() })) {
                    stack += 1;
                    stackstring = stack.to_string();
                }
                object.upgrade(upgrade.clone() + if stack == 1 { "" } else { stackstring.as_str() } );
                self.broadcast(ServerToClient::UpgradeThing (thing, upgrade.clone() + if stack == 1 { "" } else { stackstring.as_str() }));
                break;
            }
        }
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
        self.objects[i].exposed_properties.physics.shape.w = w;
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
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0.0..self.gamesize);
        let y = rng.gen_range(0.0..self.gamesize);
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
        if self.vvlm {
            return; // rubble can't be placed in VVLMs, this is a preservation mechanism
        }
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
        self.objects[i].exposed_properties.physics.velocity = velocity;
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

    fn deal_with_one_object(&mut self, x : usize, y : usize) {
        if x == y {
            println!("SYSTEM BROKE BECAUSE X EQUALS Y! CHECK YOUR MATH!");
        }
        if self.objects[x].exposed_properties.carrier_properties.is_carried || self.objects[y].exposed_properties.carrier_properties.is_carried {
            return; // Never do any kind of collisions on carried objects.
        }
        let intasectah = self.objects[x].exposed_properties.physics.shape().intersects(self.objects[y].exposed_properties.physics.shape());
        if intasectah.0 {
            if self.objects[x].exposed_properties.carrier_properties.will_carry(self.objects[y].identify()) {
                self.carry_tasks(x, y);
                return;
            }
            if self.objects[y].exposed_properties.carrier_properties.will_carry(self.objects[x].identify()) {
                self.carry_tasks(y, x);
                return;
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

    fn zone_check(&self, shape : BoxShape) -> Vec<usize> {
        let zonesize = self.gamesize / self.worldzone_count as f32;
        let mut ret = vec![];
        for x in 0..self.worldzone_count {
            for y in 0..self.worldzone_count {
                let zone_box = BoxShape::from_corners(x as f32 * zonesize, y as f32 * zonesize, (x as f32 + 1.0) * zonesize, (y as f32 + 1.0) * zonesize);
                if zone_box.intersects(shape).0 {
                    ret.push(x + y * self.worldzone_count);
                }
            }
        }
        ret
    }

    fn deal_with_objects(&mut self) {
        if self.objects.len() == 0 { // shawty circuit
            return;
        }
        if self.worldzone_count == 1 { // classic code if it's a one-zone world
            for x in 0..(self.objects.len() - 1) { // Go from first until the next-to-last item, because the inner loop goes from second to last.
                for y in (x + 1)..self.objects.len() {
                    self.deal_with_one_object(x, y);
                }
            }
        }
        else {
            for i in 0..self.objects.len() {
                for zone in &self.objects[i].zones {
                    if !self.zones[*zone].contains(&i) {
                        self.zones[*zone].push(i);
                    }
                }
            }
            for zone in 0..self.zones.len() {
                if self.zones[zone].len() > 1 { // if there's only one object in the zone, it can't hit anything, and if there are no objects...
                    for x in 0..(self.zones[zone].len() - 1) {
                        for y in (x + 1)..self.zones[zone].len() {
                            if x == y {
                                println!("Something very bad happened");
                            }
                            else {
                                self.deal_with_one_object(self.zones[zone][x], self.zones[zone][y]);
                            }
                        }
                    }
                }
                self.zones[zone].clear(); // doesn't actually deallocate, just sets len to 0.
            }
        }
        for i in 0..self.objects.len() {
            if self.objects[i].exposed_properties.physics.translated() || self.objects[i].exposed_properties.physics.rotated() || self.objects[i].exposed_properties.physics.resized() {
                self.objects[i].zones = self.zone_check(self.objects[i].exposed_properties.physics.shape);
            }
        }
        // the new zones code CAUSES a bug where objects outside of the world boundary don't collide with anything.
    }

    fn send_physics_updates(&mut self) {
        let mut i : usize = 0;
        while i < self.objects.len() {
            //let mut args_vec = vec![self.objects[i].get_id().to_string()];
            let id = self.objects[i].get_id();
            let phys = self.objects[i].get_physics_object();
            if phys.translated() || phys.rotated() || phys.resized() {
                let command = ServerToClient::MoveObjectFull (id, phys.shape.x, phys.shape.y, phys.shape.a, phys.shape.w, phys.shape.h);
                self.broadcast(command);
            }
            /*DON'T DELETE THIS
            if phys.rotated() || phys.resized() {
                args_vec.push(phys.angle().to_string());
            }
            if phys.resized() {
                args_vec.push(phys.width().to_string());
                args_vec.push(phys.height().to_string());
            }
            if args_vec.len() > 1 {
                self.broadcast(P rotocolMessage {
                    command: 'M',
                    args: args_vec
                });
            }*/
            unsafe {
                (*(&mut self.objects as *mut Vec<GamePieceBase>))[i].update(self);
            }
            // Do death checks a bit late (pun not intended) so objects have a chance to self-rescue.
            if self.objects[i].dead() {
                unsafe {
                    (*(&mut self.objects as *mut Vec<GamePieceBase>))[i].die(self);
                }
                self.broadcast(ServerToClient::Delete (self.objects[i].get_id()));
                self.objects.remove(i);
                continue; // don't allow it to reach the increment
            }
            i += 1;
        }
    }

    fn delete_obj(&mut self, id : u32) {
        match self.obj_lookup(id) {
            Some (index) => {
                self.broadcast(ServerToClient::Delete (id));
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
                self.broadcast(ServerToClient::Delete (self.objects[i].get_id()));
                self.objects.remove(i);
                delted = true;
            }
            if !delted { // WASTING a SINGLE CYCLE. hee hee hee.
                i += 1;
            }
        }
    }

    fn mainloop(&mut self) {
        if self.authenticateds == 0 { // nothing happens if there isn't anyone for it to happen to
            return;
        }
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
                        self.broadcast(ServerToClient::Tick (self.autonomous.as_ref().unwrap().2, 2));
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
                    self.broadcast(ServerToClient::Tie);
                    println!("Tie broadcast complete.");
                    self.reset();
                }
                else {
                    for team in &self.teams {
                        if team.live_count == self.living_players { // If one team holds all the players
                            println!("GAME ENDS WITH A WINNER");
                            self.broadcast(ServerToClient::End (team.banner_id as u32));
                            self.reset();
                            return;
                        }
                    }
                    if self.living_players == 1 {
                        println!("GAME ENDS WITH A WINNER");
                        self.broadcast(ServerToClient::End (self.winning_banner as u32));
                        self.reset();
                    }
                }
            }
            if self.mode == GameMode::Play {
                self.send_physics_updates();
            }
            self.broadcast_tx.send(ClientCommand::Tick (self.counter, self.mode)).expect("Broadcast failed");
            if self.mode == GameMode::Play {
                self.deal_with_objects();
                self.place_timer -= 1;
                if self.place_timer <= 0 {
                    self.place_timer = rand::random::<u32>() % 200 + 50; // set to 2 for object count benchmarking
                    self.place_random_rubble();
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

    fn broadcast<'a>(&'a self, message : ServerToClient) {
        self.broadcast_tx.send(ClientCommand::Send (message)).expect("Broadcast failed");
    }

    fn chat(&self, content : String, sender : usize, priority : u8, to_whom : Option<usize>) {
        self.broadcast_tx.send(ClientCommand::ChatRoom (content, sender, priority, to_whom)).expect("Chat message failed");
    }

    fn add(&mut self, mut piece : GamePieceBase, banner : Option<usize>) -> u32 {
        piece.set_id(self.top_id);
        self.top_id += 1;
        piece.zones = self.zone_check(piece.exposed_properties.physics.shape);
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

    /*fn banner_add(&mut self, mut dispatcha : Option<&mut Client>, mut banner : Arc<String>) -> usize {
        while self.banners.contains(&banner) {
            banner = Arc::new(banner.to_string() + ".copy");
        }
        let bannah = self.banners.len();
        println!("Created new banner {}, {}", self.banners.len(), banner);
        self.broadcast(ServerToClient::BannerAdd (bannah as u32, banner.to_string()));
        if dispatcha.is_some() {
            dispatcha.as_mut().unwrap().banner = self.banners.len();
            println!("Added the banner to a client");
            if dispatcha.as_ref().unwrap().team.is_some() {
                self.broadcast(ServerToClient::BannerAddToTeam (bannah as u32, self.teams[dispatcha.as_ref().unwrap().team.unwrap()].banner_id as u32));
            }
        }
        self.banners.push(banner.clone());
        bannah
    }*/

    fn banner_add(&mut self, mut banner : String) -> usize {
        while self.banners.contains(&banner) {
            banner += ".copy";
        }
        let bannah = self.banners.len();
        self.broadcast(ServerToClient::BannerAdd (bannah as u32, banner.clone()));
        println!("Created new banner {}, {}", bannah, banner);
        self.banners.push(banner);
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

    /*async fn metadata(&mut self, user : &mut Client) {
        println!("Sending metadata to {}", self.banners[user.banner]);
        for index in 0..self.banners.len() {
            let banner = &self.banners[index];
            let team = self.get_team_of_banner(index);
            println!("Team: {:?}", team);
            user.send_protocol_message(ServerToClient::BannerAdd (index as u32, banner.to_string())).await;
            if team.is_some(){
                user.send_protocol_message(ServerToClient::BannerAddToTeam (index as u32, self.teams[team.unwrap()].banner_id as u32)).await;
            }
        }
        for piece in &self.objects {
            user.send_protocol_message(piece.get_new_message()).await;
            for i in 0..piece.upgrades.len() {
                let upg = piece.upgrades[i].to_string();
                user.send_protocol_message(ServerToClient::UpgradeThing(piece.get_id(), upg)).await;
            }
        }
        user.send_protocol_message(ServerToClient::Metadata (self.gamesize, self.terrain_seed)).await;
    }

    async fn user_logged_in(&mut self, user : &mut Client) {
        self.authenticateds += 1;
        self.metadata(user).await;
    }

    async fn spectator_joined(&mut self, user : &mut Client) {
        self.metadata(user).await;
    }*/

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
        let banner = self.banner_add(name);
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


impl Client {
    fn new(socket : WebSocketClientStream, commandah : tokio::sync::mpsc::Sender<ServerCommand>) -> Self {
        Self {
            socket,
            game_cmode: GameMode::Waiting,
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
            a2a: 0,
            walls_cap : 2,
            walls_remaining : 4 // you get a bonus on turn 1
        }
    }

    async fn grant_a2a(&mut self) {
        self.a2a += 1;
        self.refresh_a2a().await;
    }

    async fn collect(&mut self, amount : i32) {
        self.score += amount;
        self.send_protocol_message(ServerToClient::SetScore(self.score)).await;
    }

    async fn send_protocol_message(&mut self, message : ServerToClient) {
        match self.socket.send(message).await { // unwrap the error; it isn't important here
            _ => {}
        }
    }

    async fn refresh_a2a(&mut self) {
        self.send_protocol_message(ServerToClient::A2A (self.a2a)).await;
    }

    async fn cost(&mut self, amount : i32) -> bool {
        if self.score >= amount {
            self.collect(-amount).await;
            true
        }
        else {
            self.kys = true;
            println!("Client can't afford to buy a thing!");
            false
        }
    }

    async fn handle(&mut self, message : ClientToServer) {
        if self.is_authorized {
            match message {
                ClientToServer::Place (x, y, tp) => {
                    if self.places_this_turn >= 30 {
                        return;
                    }
                    if self.game_cmode == GameMode::Play && tp != b'c' { // if it is trying to place an object, but it isn't strat mode or waiting mode and it isn't placing a castle
                        // originally, this retaliated, but now it just refuses. the retaliation was a problem.
                        println!("ATTEMPT TO PLACE IN PLAY MODE");
                        return; // can't place things if it ain't strategy. Also this is poison.
                    }
                    self.places_this_turn += 1;/*
                    if x < 0.0 || x > server.gamesize || y < 0.0 || y > server.gamesize { 
                        self.kys = true;
                        return;
                    }*/
                    // KNOWN UNFIXED VULNERABILITY: this code was dumb, we need to figure out a better way to handle it. it's not a *serious* problem that richard can place outside of the map.
                    match tp {
                        b'c' => {
                            if !self.has_placed {
                                self.has_placed = true;
                                self.commandah.send(ServerCommand::Place (PlaceCommand::Castle (x, y, self.mode, self.banner, self.team))).await.unwrap();
                            }
                            else {
                                println!("User is attempting to place a castle more than once.");
                                self.kys = true;
                            }
                        },
                        b'w' => {
                            if self.walls_remaining > 0 {
                                self.commandah.send(ServerCommand::Place (PlaceCommand::SimplePlace (x, y, Some(self.banner), b'w'))).await.unwrap();
                                self.walls_remaining -= 1;
                            }
                        },
                        b'F' => {
                            match self.m_castle {
                                Some(cid) => {
                                    self.commandah.send(ServerCommand::Place (PlaceCommand::Fort (x, y, Some(self.banner), cid))).await.unwrap();
                                }
                                None => {}
                            }
                        },
                        _ => {
                            self.commandah.send(ServerCommand::Place (PlaceCommand::SimplePlace (x, y, Some(self.banner), tp))).await.unwrap();
                        }
                    }
                },
                ClientToServer::Cost (amount) => {
                    let amount = amount.abs();
                    println!("Costing an amount of money equal to {amount} coins");
                    self.collect(-amount).await;
                },
                ClientToServer::Move (id, x, y, a) => {
                    self.commandah.send(ServerCommand::Move (self.banner, id, x, y, a)).await.unwrap();
                },
                ClientToServer::LaunchA2A (target) => { // AIR TO AIR!
                    if self.a2a == 0 {
                        self.kys = true;
                    }
                    if self.m_castle.is_some() {
                        self.a2a -= 1;
                        self.refresh_a2a().await;
                        self.commandah.send(ServerCommand::Place (PlaceCommand::A2A (self.m_castle.unwrap(), target, self.banner))).await.unwrap();
                    }
                },
                ClientToServer::PilotRTF (fire, left, right, airbrake, shoot) => {
                    if self.game_cmode == GameMode::Play && self.m_castle.is_some(){
                        self.commandah.send(ServerCommand::PilotRTF (self.m_castle.unwrap(), fire, left, right, airbrake, shoot)).await.unwrap();
                    }
                },
                ClientToServer::Chat (chatter, broadcast) => { // Talk
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
                    self.commandah.send(ServerCommand::Chat (self.banner, chatter, if self.is_team_leader { 1 } else { 0 },
                        if self.team.is_none() || broadcast {
                            None
                        }
                        else {
                            self.team
                        }
                    )).await.unwrap();
                },
                ClientToServer::UpgradeThing (_, _) => {
                    // Upgrade.
                    /*
                    println!("Upgrading id {} to {}", id, upgrade);
                    let price = *server.upg_costs.entry(upgrade.clone()).or_insert(0);
                    if self.score >= price {
                        self.collect(-price).await;
                        for object in &mut server.objects {
                            if object.get_id() == id {
                                object.upgrade(upgrade.clone());
                                server.broadcast(ServerToClient::UpgradeThing (id, upgrade));
                                break;
                            }
                        }
                    }
                    else { // something nefarious is going on!
                        self.kys = true;
                    }*/
                    self.send_protocol_message(ServerToClient::Chat ("Upgrading manually is DISABLED! Use the Shop command instead.".to_string(), 0, 6)).await;
                    self.kys = true;
                },
                ClientToServer::Ping => {
                    self.send_protocol_message(ServerToClient::Pong).await;
                },
                ClientToServer::Connect (_, _, _) => {
                    println!("What? Client trying to connect twice? Killing.");
                    self.kys = true;
                },
                ClientToServer::SelfTest (boolean, byte, ushort, uint, int, float, string) => {
                    println!("Got self test message {}, {}, {}, {}, {}, {}, {}", boolean, byte, ushort, uint, int, float, string);
                },
                ClientToServer::Shop (thing) => {
                    if self.m_castle.is_some() {
                        match thing {
                            b'w' => {
                                if self.cost(30).await {
                                    self.walls_cap += 2;
                                    self.walls_remaining += 2;
                                }
                            }
                            b'g' => {
                                if self.cost(30).await {
                                    self.commandah.send(ServerCommand::UpgradeNextTier (self.m_castle.unwrap(), "b".to_string())).await.unwrap();
                                }
                            }
                            b's' => {
                                if self.cost(40).await {
                                    self.commandah.send(ServerCommand::UpgradeNextTier (self.m_castle.unwrap(), "s".to_string())).await.unwrap();
                                }
                            }
                            b'f' => {
                                if self.cost(70).await {
                                    self.commandah.send(ServerCommand::UpgradeNextTier (self.m_castle.unwrap(), "f".to_string())).await.unwrap();
                                }
                            }
                            b'h' => {
                                if self.cost(150).await {
                                    self.commandah.send(ServerCommand::UpgradeNextTier (self.m_castle.unwrap(), "h".to_string())).await.unwrap();
                                }
                            }
                            b'a' => {
                                if self.cost(100).await {
                                    self.grant_a2a().await;
                                }
                            }
                            _ => {
                                println!("Invalid shop command {}", thing);
                            }
                        }
                    }
                }
            }
        }
        else {
            match message {
                ClientToServer::Connect (password, banner, mode) => {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
                    self.commandah.send(ServerCommand::BeginConnection (password, banner, mode.clone(), tx)).await.unwrap();
                    loop {
                        match rx.recv().await {
                            Some(command) => {
                                match command {
                                    InitialSetupCommand::Joined (authstate) => {
                                        match authstate {
                                            AuthState::Spectator => {
                                                self.send_protocol_message(ServerToClient::YouAreSpectating).await;
                                            },
                                            AuthState::Team (team, tl) => {
                                                self.send_protocol_message(ServerToClient::Welcome).await;
                                                self.team = Some(team);
                                                self.is_team_leader = tl;
                                                if tl {
                                                    self.send_protocol_message(ServerToClient::YouAreTeamLeader).await;
                                                }
                                                self.is_authorized = true;
                                            },
                                            AuthState::Single => {
                                                self.send_protocol_message(ServerToClient::Welcome).await;
                                                self.is_authorized = true;
                                            },
                                            _ => {println!("yooo");}
                                        }
                                    },
                                    InitialSetupCommand::Message (message) => {
                                        self.send_protocol_message(message).await;
                                    },
                                    InitialSetupCommand::Metadata (gamesize, banner) => {
                                        self.send_protocol_message(ServerToClient::Metadata(gamesize, banner as u32)).await;
                                        self.banner = banner;
                                    },
                                    InitialSetupCommand::Finished => {
                                        break;
                                    }
                                }
                            }
                            None => {
                                panic!("PANICCCC");
                            }
                        }
                    }
                    self.mode = match mode.as_str() {
                        "normal" => ClientMode::Normal,
                        "defender" => ClientMode::Defense,
                        "rtf" => ClientMode::RealTimeFighter,
                        _ => ClientMode::Normal
                    };
                    /*
                    if server.new_user_can_join() {
                        let lockah = server.authenticate(password, mode == "spectator");
                        match lockah { // If you condense this, for === RUST REASONS === it keeps the mutex locked.
                            AuthState::Error => {
                                println!("New user has invalid password!");
                                self.send_protocol_message(ServerToClient::BadPassword).await;
                                return;
                            },
                            AuthState::Single => {
                                println!("New user has authenticated as single player");
                                self.send_protocol_message(ServerToClient::Welcome).await;
                                server.user_logged_in(self).await;
                                server.banner_add(Some(self), Arc::new(banner));
                                self.is_authorized = true;
                            },
                            AuthState::Team (team, tl) => {
                                println!("New user has authenticated as player in team {}", server.banners[server.teams[team].banner_id]);
                                self.team = Some(team);
                                self.send_protocol_message(ServerToClient::Welcome).await;
                                if tl {
                                    self.send_protocol_message(ServerToClient::YouAreTeamLeader).await;
                                    self.is_team_leader = true;
                                }
                                server.user_logged_in(self).await;
                                server.banner_add(Some(self), Arc::new(banner));
                                server.teams[team].members.push(self.banner);
                                self.is_authorized = true;
                            },
                            AuthState::Spectator => {
                                println!("Spectator joined!");
                                self.send_protocol_message(ServerToClient::YouAreSpectating).await;
                                server.spectator_joined(self).await;
                            }
                        }
                    }
                    else {
                        println!("New user can't join!");
                        self.send_protocol_message(ServerToClient::YouAreSpectating).await;
                        server.spectator_joined(self).await;
                    }*/
                },
                ClientToServer::Ping => {
                    self.send_protocol_message(ServerToClient::Pong).await;
                },
                ClientToServer::SelfTest (boolean, byte, ushort, uint, int, float, string) => {
                    println!("Got self test message {}, {}, {}, {}, {}, {}, {}", boolean, byte, ushort, uint, int, float, string);
                }
                _ => {
                    println!("Client appears to be making a shoddy attempt to suborn. Deleting.");
                    self.kys = true;
                }
            }
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


async fn got_client(client : WebSocketClientStream, server : Arc<Mutex<Server>>, broadcaster : tokio::sync::broadcast::Sender<ClientCommand>, commandset : tokio::sync::mpsc::Sender<ServerCommand>){
    commandset.send(ServerCommand::Connect).await.unwrap();
    let mut receiver = broadcaster.subscribe();
    let mut moi = Client::new(client, commandset);/*
    if server.lock().await.passwordless {
        moi.send_singlet('p').await;
    }*/
    // passwordless broadcasts aren't really relevant any more
    'cliloop: loop {
        select! {
            insult = moi.socket.read::<ClientToServer>().fuse() => {
                match insult {
                    Some(insult) => {
                        moi.handle(insult).await;
                        if moi.kys { // if it's decided to break the connection
                            break 'cliloop;
                        }
                    }
                    None => {
                        break 'cliloop;
                    }
                }
            },
            command = receiver.recv().fuse() => {
                match command {
                    Ok (ClientCommand::Tick (counter, mode)) => {
                        moi.game_cmode = mode;
                        if mode == GameMode::Play { // if it's play mode
                            moi.places_this_turn = 0;
                            moi.walls_remaining = moi.walls_cap; // it can't use 'em until next turn, ofc
                        }
                        let mut schlock = server.lock().await;
                        schlock.winning_banner = moi.banner;
                        moi.send_protocol_message(ServerToClient::Tick (counter, match mode {
                            GameMode::Play => 0, 
                            GameMode::Strategy => 1,
                            GameMode::Waiting => 2
                        })).await;
                        for obj in &schlock.objects {
                            if obj.get_banner() == moi.banner && obj.do_stream_health() {
                                moi.send_protocol_message(ServerToClient::HealthUpdate (obj.get_id(), obj.get_health_perc())).await;
                            }
                        }
                        if !moi.is_alive(schlock) {
                            if moi.m_castle.is_some() {
                                moi.m_castle = None;
                                moi.send_protocol_message(ServerToClient::YouLose).await;
                                moi.commandah.send(ServerCommand::LivePlayerDec (moi.team, moi.mode)).await.expect("Broadcast failed");
                            }
                        }
                    },
                    Ok (ClientCommand::Send (message)) => {
                        moi.send_protocol_message(message).await;
                    },
                    Ok (ClientCommand::SetCastle (banner, id)) => {
                        if moi.banner == banner {
                            moi.m_castle = Some(id);
                        }
                    },
                    /*Ok (ClientCommand::SendToTeam (message, team)) => {
                        if moi.team.is_some() && moi.team.unwrap() == team {
                            moi.send_protocol_message(message).await;
                        }
                    },*/
                    Ok (ClientCommand::ScoreTo (banner, amount)) => {
                        if moi.banner == banner {
                            moi.collect(amount).await;
                        }
                    },
                    Ok (ClientCommand::CloseAll) => {
                        break 'cliloop;
                    },
                    Ok (ClientCommand::ChatRoom (content, sender, priority, target)) => {
                        if target.is_none() || target == moi.team {
                            moi.send_protocol_message(ServerToClient::Chat(content, sender as u32, priority)).await;
                        }
                    },
                    Ok (ClientCommand::GrantA2A (to)) => {
                        if to == moi.banner && moi.mode == ClientMode::RealTimeFighter {
                            moi.grant_a2a().await;
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
                                moi.send_protocol_message(ServerToClient::Add (id)).await;
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
    moi.socket.shutdown().await;
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


#[derive(Copy, Clone, Debug)]
pub enum PlaceCommand {
    SimplePlace (f32, f32, Option<usize>, u8), // x, y, banner, type
    Fort (f32, f32, Option<usize>, u32), // x, y, banner, item to attach the fort to
    Castle (f32, f32, ClientMode, usize, Option<usize>), // x, y, mode, banner, team
    A2A (u32, u32, usize) // gunner, target, banner
}


#[derive(Debug)]
enum InitialSetupCommand {
    Message (ServerToClient), // send an arbitrary protocol message
    Finished,
    Metadata (f32, usize),
    Joined (AuthState)
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
    RejectObject (u32),
    PrintBanners,
    Nuke (usize),
    Reset,
    Place (PlaceCommand),
    Move (usize, u32, f32, f32, f32), // banner, id, x, y, a
    PilotRTF (u32, bool, bool, bool, bool, bool),
    Chat (usize, String, u8, Option<usize>),
    UpgradeNextTier (u32, String),
    BeginConnection (String, String, String, tokio::sync::mpsc::Sender<InitialSetupCommand>) // password, banner, mode, outgoing pipe. god i've got to clean this up. vomiting face.
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
        banners             : vec!["Syst3m".to_string()],
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
        worldzone_count     : 1,
        zones               : Vec::new(),
        vvlm                : false
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
            let mut lawk = server_mutex_loopah.lock().await;
            select! { // THIS IS BROKEN! It has constant, slow locking that bogs down performance. The reason for this is mutexes.
/*
| Problems caused
|    by mutexes
|      ____
|     |    |
|     |    |
|     |    |
|     |    |
|     |    | Problems solved
|     |    |    by mutexes
|     |    |       ____
|_____|____|______|____|______
*/
                _ = interval.tick() => {
                    use tokio::time::Instant;
                    let start = Instant::now();
                    lawk.mainloop();
                    if start.elapsed() > tokio::time::Duration::from_millis((1000.0/FPS) as u64) {
                        println!("LOOP OVERRUN!");
                    }
                },
                command = commandget.recv() => {
                    match command {
                        Some (ServerCommand::Start) => {
                            lawk.start();
                        },
                        Some (ServerCommand::RejectObject (id)) => {
                            lawk.delete_obj(id);
                        },
                        Some (ServerCommand::Flip) => {
                            lawk.flip();
                        },
                        Some (ServerCommand::TeamNew (name, password)) => {
                            lawk.new_team(name, password);
                        },
                        Some (ServerCommand::Autonomous (min_players, max_players, auto_timeout)) => {
                            lawk.autonomous = Some((min_players, max_players, auto_timeout, auto_timeout));
                        },
                        Some (ServerCommand::Move (banner, id, x, y, a)) => {
                            for object in &mut lawk.objects {
                                if object.get_id() == id && object.get_banner() == banner {
                                    object.exposed_properties.goal_x = x;
                                    object.exposed_properties.goal_y = y;
                                    object.exposed_properties.goal_a = a;
                                }
                            }
                        },
                        Some (ServerCommand::UpgradeNextTier (item, upgrade)) => {
                            lawk.upgrade_next_tier(item, upgrade);
                        },
                        Some (ServerCommand::PilotRTF (id, fire, left, right, airbrake, shoot)) => {
                            match lawk.obj_lookup(id) {
                                Some (index) => {
                                    let mut thrust = 0.0;
                                    let mut resistance = 1.0;
                                    let mut angle_thrust = 0.0;
                                    let is_better_turns = lawk.objects[index].upgrades.contains(&"f3".to_string());
                                    let angle_thrust_power = if is_better_turns { 0.04 } else { 0.02 };
                                    if fire {
                                        thrust = 2.0;
                                    }
                                    if left {
                                        angle_thrust -= angle_thrust_power;
                                    }
                                    if right {
                                        angle_thrust += angle_thrust_power;
                                    }
                                    if airbrake {
                                        resistance = 0.8;
                                    }
                                    lawk.objects[index].exposed_properties.shooter_properties.suppress = !shoot;
                                    let thrust = Vector2::new_from_manda(thrust, lawk.objects[index].exposed_properties.physics.angle() - PI/2.0);
                                    lawk.objects[index].exposed_properties.physics.velocity += thrust;
                                    lawk.objects[index].exposed_properties.physics.velocity *= resistance;
                                    lawk.objects[index].exposed_properties.physics.angle_v += angle_thrust;
                                    lawk.objects[index].exposed_properties.physics.angle_v *= if is_better_turns { 0.8 } else { 0.9 };
                                    lawk.objects[index].exposed_properties.physics.angle_v *= resistance;
                                }
                                None => {}
                            }
                        },
                        Some (ServerCommand::BeginConnection (password, banner, mode, transmit)) => {
                            if lawk.new_user_can_join() {
                                let thing = lawk.authenticate(password, mode == "spectator");
                                match thing {
                                    AuthState::Error => {
                                        println!("Authentication error");

                                    },
                                    AuthState::Single => {
                                        println!("Single player joined");
                                        transmit.send(InitialSetupCommand::Joined (AuthState::Single)).await.unwrap();
                                    },
                                    AuthState::Team (_,_) => {
                                        println!("Team player joined");
                                        transmit.send(InitialSetupCommand::Joined (thing)).await.unwrap();
                                    },
                                    AuthState::Spectator => {
                                        println!("Spectator joined");
                                        transmit.send(InitialSetupCommand::Joined (AuthState::Spectator)).await.unwrap();
                                    }
                                }
                            }
                            else {
                                transmit.send(InitialSetupCommand::Joined (AuthState::Spectator)).await.unwrap();
                            }
                            for object in &lawk.objects {
                                transmit.send(InitialSetupCommand::Message (object.get_new_message())).await.unwrap();
                                for upg in &object.upgrades {
                                    transmit.send(InitialSetupCommand::Message (ServerToClient::UpgradeThing(object.exposed_properties.id, upg.clone()))).await.unwrap();
                                }
                            }
                            for i in 0..lawk.banners.len() {
                                transmit.send(InitialSetupCommand::Message (ServerToClient::BannerAdd(i as u32, lawk.banners[i].clone()))).await.unwrap();
                            }
                            let banner_id = lawk.banner_add(banner);
                            lawk.authenticateds += 1;
                            transmit.send(InitialSetupCommand::Metadata (lawk.gamesize, banner_id)).await.unwrap();
                            transmit.send(InitialSetupCommand::Finished).await.unwrap();
                        },
                        Some (ServerCommand::IoModeToggle) => {
                            lawk.is_io = !lawk.is_io;
                            println!("Set io mode to {}", lawk.is_io);
                        },
                        Some (ServerCommand::Disconnect) => {
                            lawk.clients_connected -= 1;
                            if lawk.clients_connected == 0 {
                                lawk.clear_banners();
                            }
                        },
                        Some (ServerCommand::Connect) => {
                            lawk.clients_connected += 1;
                        },
                        Some (ServerCommand::Broadcast (message)) => {
                            lawk.chat(message, 0, 6, None);
                        },
                        Some (ServerCommand::Chat (banner, message, priority, to_whom)) => {
                            println!("{} says {}", lawk.banners[banner], message);
                            lawk.chat(message, banner, priority, to_whom);
                        }
                        Some (ServerCommand::PasswordlessToggle) => {
                            lawk.passwordless = !lawk.passwordless;
                            lawk.broadcast(ServerToClient::SetPasswordless (lawk.passwordless));
                            println!("Set passwordless mode to {}", lawk.passwordless);
                        },
                        Some (ServerCommand::LivePlayerInc (team, mode)) => {
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
                        Some (ServerCommand::LivePlayerDec (team, mode)) => {
                            if mode != ClientMode::RealTimeFighter {
                                lawk.isnt_rtf -= 1;
                            }
                            lawk.living_players -= 1;
                            if team.is_some() {
                                lawk.teams[team.unwrap()].live_count -= 1;
                            }
                            println!("Player died. Living players: {}", lawk.living_players);
                        },
                        Some (ServerCommand::PrintBanners) => {
                            println!("Current banners are,");
                            for banner in 0..lawk.banners.len() {
                                println!("{}: {}", banner, lawk.banners[banner]);
                            }
                        }
                        Some (ServerCommand::Nuke (banner)) => {
                            for object in 0..lawk.objects.len() {
                                if lawk.objects[object].get_banner() == banner {
                                    let x = lawk.objects[object].exposed_properties.physics.cx();
                                    let y = lawk.objects[object].exposed_properties.physics.cy();
                                    lawk.place_nuke(x, y, 0.0, None);
                                }
                            }
                        },
                        Some (ServerCommand::Reset) => {
                            lawk.reset()
                        },
                        Some (ServerCommand::Place (PlaceCommand::SimplePlace (x, y, banner, tp))) => {
                            match tp {
                                b'f' => {
                                    lawk.place_basic_fighter(x, y, 0.0, banner);
                                },
                                b'm' => {
                                    lawk.place_mls(x, y, 0.0, banner);
                                },
                                b'a' => {
                                    lawk.place_antirtf_missile(x, y, 0.0, banner);
                                },
                                b'K' => {
                                    lawk.place_carrier(x, y, 0.0, banner);
                                },
                                b't' => {
                                    lawk.place_tie_fighter(x, y, 0.0, banner);
                                },
                                b's' => {
                                    lawk.place_sniper(x, y, 0.0, banner);
                                },
                                b'h' => {
                                    lawk.place_missile(x, y, 0.0, banner);
                                },
                                b'T' => {
                                    lawk.place_turret(x, y, 0.0, banner);
                                },
                                b'n' => {
                                    lawk.place_nuke(x, y, 0.0, banner);
                                },
                                b'w' => {
                                    lawk.place_wall(x, y, banner);
                                },
                                _ => {
                                    println!("The client attempted to place an object with invalid type {}", tp);
                                }
                            }
                        }
                        Some (ServerCommand::Place (PlaceCommand::Fort (x, y, banner, target))) => {
                            match lawk.obj_lookup(target) {
                                Some(index) => {
                                    let fort = lawk.place_fort(x, y, 0.0, banner);
                                    lawk.objects[index].add_fort(fort);
                                }
                                None => {}
                            }
                        }
                        Some (ServerCommand::Place (PlaceCommand::Castle (x, y, mode, banner, team))) => {
                            if lawk.mode != GameMode::Waiting && !lawk.is_io {
                                continue;
                            }
                            lawk.costs = false;
                            let castle = lawk.place_castle(x, y, mode == ClientMode::RealTimeFighter, Some(banner));
                            lawk.broadcast_tx.send(ClientCommand::SetCastle (banner, castle)).unwrap();
                            match mode {
                                ClientMode::Normal => {
                                    lawk.place_basic_fighter(x - 200.0, y, PI, Some(banner));
                                    lawk.place_basic_fighter(x + 200.0, y, 0.0, Some(banner));
                                    lawk.place_basic_fighter(x, y - 200.0, 0.0, Some(banner));
                                    lawk.place_basic_fighter(x, y + 200.0, 0.0, Some(banner));
                                    lawk.broadcast_tx.send(ClientCommand::ScoreTo (banner, 100)).unwrap();
                                },
                                ClientMode::RealTimeFighter => {
                                    lawk.place_basic_fighter(x - 100.0, y, PI, Some(banner));
                                    lawk.place_basic_fighter(x + 100.0, y, 0.0, Some(banner));
                                    lawk.broadcast_tx.send(ClientCommand::GrantA2A (banner)).unwrap();
                                },
                                ClientMode::Defense => {
                                    lawk.place_basic_fighter(x - 200.0, y, PI, Some(banner));
                                    lawk.place_basic_fighter(x + 200.0, y, 0.0, Some(banner));
                                    lawk.place_turret(x, y - 200.0, 0.0, Some(banner));
                                    lawk.place_turret(x, y + 200.0, 0.0, Some(banner));
                                    lawk.broadcast_tx.send(ClientCommand::ScoreTo (banner, 25)).unwrap();
                                },
                                _ => {

                                }
                            }
                            // shamelessly copy/pasted from LivePlayerInc. clean up when the dust settles!
                            lawk.costs = true;
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
                        Some (ServerCommand::Place (PlaceCommand::A2A (castle, target, banner))) => {
                            let target_i = match lawk.obj_lookup(target) { Some(i) => i, None => continue };
                            let castle_i = match lawk.obj_lookup(castle) { Some(i) => i, None => continue };
                            let obj_vec = lawk.objects[target_i].exposed_properties.physics.vector_position();
                            if (lawk.objects[castle_i].exposed_properties.physics.vector_position() - obj_vec).magnitude() < 1500.0 {
                                let off_ang = functions::coterminal(lawk.objects[castle_i].exposed_properties.physics.angle() - (lawk.objects[castle_i].exposed_properties.physics.vector_position() - obj_vec).angle(), PI * 2.0);
                                let pos = lawk.objects[castle_i].exposed_properties.physics.vector_position() + Vector2::new_from_manda(if off_ang > PI { 50.0 } else { -50.0 }, lawk.objects[castle_i].exposed_properties.physics.angle());
                                let launchangle = lawk.objects[castle_i].exposed_properties.physics.angle() - PI/2.0; // rust requires this to be explicit because of the dumbass borrow checker
                                let a2a_id = lawk.place_air2air(pos.x, pos.y, launchangle, target, Some(banner));
                                let a2a_i = lawk.obj_lookup(a2a_id).unwrap(); // it's certain to exist
                                lawk.objects[a2a_i].exposed_properties.physics.velocity = lawk.objects[castle_i].exposed_properties.physics.velocity;
                            }
                        },
                        None => {
                            println!("The channel handling server control was disconnected!");
                        }
                        /*Err (TryRecvError::Disconnected) => {
                            println!("The channel handling server control was disconnected!");
                        },
                        Err (TryRecvError::Empty) => {} // Do nothing; we expect it to be empty quite often.*/
                    }
                }
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
                    "getbanners" => {
                        ServerCommand::PrintBanners
                    },
                    "nuke" => {
                        ServerCommand::Nuke (input("Banner to nuke: ").parse::<usize>().unwrap())
                    }
                    "reset" => {
                        ServerCommand::Reset
                    }
                    _ => {
                        println!("Invalid command.");
                        continue;
                    }
                };
                commandset.send(to_send).await.expect("OOOOOOPS");
            }
        });
    }

    let mut websocket_server = WebSocketServer::new(port, "MMOSG".to_string()).await;
    loop {
        let client = websocket_server.accept::<ClientToServer, ServerToClient>().await;
        tokio::task::spawn(got_client(client, server_mutex.clone(), broadcast_tx.clone(), commandset_clone.clone()));
    }
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
