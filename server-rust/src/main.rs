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
use std::sync::Arc;
use crate::gamepiece::*;
use crate::nexus::Nexus;
//use crate::nexus::NexusEnemy;
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
    is_superuser      : bool,
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
    kys               : bool,
    a2a               : u16,
    walls_remaining   : u16,
    walls_cap         : u16,
    game_cmode        : GameMode,
    is_ready          : bool
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
    Tie,
    SeedCompletion (u32, u16), // seed id, completion value
    Carry (u32, u32), // carrier, carried
    UnCarry (u32), // no longer carrying this guy
    YouAreGod // you are God
}

#[derive(ProtocolFrame, Debug, Clone)]
pub enum ClientToServer {
    Ping,
    Connect (String, String, String), // password, banner, mode
    Place (f32, f32, u8, u32), // x, y, type, variant
    Cost (i32),
    Move (u32, f32, f32, f32),
    LaunchA2A (u32),
    PilotRTF (bool, bool, bool, bool, bool),
    Chat (String, bool), // the bool is if it's sent to everyone or not
    UpgradeThing (u32, String),
    SelfTest (bool, u8, u16, u32, i32, f32, String),
    Shop (u8),
    ReadyState (bool),
    GodDelete (u32),
    GodReset,
    GodDisconnect (u32),
    GodNuke (u32),
    GodFlip
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
    members          : Vec <usize> // BANNERS in this team. Remove them when players die.
}


#[derive(Debug, Clone)]
enum ClientCommand { // Commands sent to clients
    Send (ServerToClient),
    SendTo (ServerToClient, usize),
    Tick (u32, GameMode),
    ScoreTo (usize, i32),
    CloseAll,
    ChatRoom (String, usize, u8, Option<usize>), // message, sender, priority
    GrantA2A (usize),
    AttachToBanner (u32, usize, i32),
    SetCastle (usize, u32), // banner to set, id of the castle
    HealthStream (u32, f32), // id to stream, health value
    RoleCall, // the client will immediately report its banner in the WinningBanner message.
    SomeoneDied (usize), // banner
    Christmas,
    Close (usize)
}


pub struct Server {
    admin_password    : String,
    self_test         : bool,
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
    isnt_rtf          : u32,
    times             : (f32, f32),
    clients_connected : u32,
    is_headless       : bool,
    permit_npcs       : bool,
    port              : u16,
    sql               : String,
    worldzone_count   : usize,
    zones             : Vec<Vec<usize>>,
    vvlm              : bool,
    readies           : u32
}

#[derive(Debug)]
enum AuthState {
    Error,
    Single,
    Team (usize, bool),
    Spectator,
    God
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

    fn object_field_check(&self, object : BoxShape, x : f32, y : f32, radius : f32) -> bool { // radius is usually 800.0 because 400.0 to a side.
        object.ong_fr().bigger(radius).contains(Vector2::new(x, y)) // this ong frs it first because contains doesn't really work on rotated objects
    }

    fn is_inside_friendly(&self, x : f32, y : f32, banner : usize, tp : char, field_width : f32) -> bool {
        for obj in &self.objects {
            if obj.identify() == tp {
                if obj.get_banner() == banner {
                    if self.object_field_check(obj.exposed_properties.physics.shape, x, y, field_width) {
                        return true; // short circuit
                    }
                }
            }
        }
        return false;
    }

    pub fn stream_health(&mut self, id : u32, health : f32) {
        self.broadcast_tx.send(ClientCommand::HealthStream (id, health)).unwrap();
    }

    /*fn upgrade_thing_to(&mut self, thing : u32, upgrade : String) {
        for object in &mut self.objects {
            if object.get_id() == thing {
                object.upgrade(upgrade.clone());
                self.broadcast(ServerToClient::UpgradeThing (thing, upgrade));
                break;
            }
        }
    }*/

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
            if self.object_field_check(obj.exposed_properties.physics.shape, x, y, 800.0) {
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
                ReqZone::WithinCastle => {
                    self.is_inside_friendly(x, y, banner, 'c', 1600.0) || self.is_inside_friendly(x, y, banner, 'R', 800.0)
                },
                ReqZone::WithinCastleOrFort => {
                    self.is_inside_friendly(x, y, banner, 'c', 1600.0) || self.is_inside_friendly(x, y, banner, 'F', 800.0) || self.is_inside_friendly(x, y, banner, 'R', 800.0)
                },
                ReqZone::AwayFromThings => {
                    self.is_clear(x, y)
                },
                ReqZone::Both => {
                    self.is_clear(x, y) || self.is_inside_friendly(x, y, banner, 'c', 1600.0) || self.is_inside_friendly(x, y, banner, 'F', 800.0)
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

    fn score_to(&self, banner : usize, amount : i32) {
        self.broadcast_tx.send(ClientCommand::ScoreTo(banner, amount)).expect("BROADCAST FAILLEDDDDDD");
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

    fn place_chest(&mut self, x : f32, y : f32, sender : Option<usize>) {
        self.place(Box::new(Chest::new()), x, y, 0.0, sender);
    }

    fn place_seed(&mut self, x : f32, y : f32, sender : Option<usize>) {
        self.place(Box::new(Seed::new()), x, y, 0.0, sender);
    }

    fn place_green_thumb(&mut self, x : f32, y : f32, sender : Option<usize>) {
        self.place(Box::new(GreenThumb::new()), x, y, 0.0, sender);
    }

    fn place_gold_bar(&mut self, x : f32, y : f32, sender : Option<usize>) -> u32 {
        self.place(Box::new(GoldBar::new()), x, y, 0.0, sender)
    }

    fn place_nexus(&mut self, x : f32, y : f32, effect_radius : f32) -> u32 {
        self.place(Box::new(Nexus::new(effect_radius)), x, y, 0.0, None)
    }

    /*fn place_nexus_enemy(&mut self, x : f32, y : f32, parent : u32) -> u32 {
        self.place(Box::new(NexusEnemy::new(parent)), x, y, rand::random::<f32>() % (PI * 2.0), None)
    }*/

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
        let chance = rand::random::<u16>() % 100;
        let thing : Box<dyn GamePiece + Send + Sync> = {
            if chance < 20 {
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

    pub fn into_berth(&mut self, carrier : usize, id : u32, berth : usize) {
        let thing = self.obj_lookup(id).unwrap();
        self.carry_tasks(carrier, thing);
        self.objects[thing].exposed_properties.carrier_properties.berth = berth;
        unsafe { // gotta hate borrow checking
            //let mut objects = *(&mut self.objects as *mut Vec<GamePieceBase>);
            (*(&mut self.objects as *mut Vec<GamePieceBase>))[carrier].update_carried(self);
        }
        let phys = self.objects[thing].exposed_properties.physics.shape;
        self.broadcast(ServerToClient::MoveObjectFull (id, phys.x, phys.y, phys.a, phys.w, phys.h));
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
        self.objects[carried].exposed_properties.carrier_properties.carrier = self.objects[carrier].get_id();
        let mut carrier_props = self.objects[carrier].exposed_properties.clone();
        let mut carried_props = self.objects[carried].exposed_properties.clone();
        unsafe { // gotta hate borrow checking
            //let mut objects = *(&mut self.objects as *mut Vec<GamePieceBase>);
            (*(&mut self.objects as *mut Vec<GamePieceBase>))[carrier].piece.on_carry(&mut carrier_props, &mut carried_props, self);
        }
        self.objects[carrier].exposed_properties = carrier_props;
        self.objects[carried].exposed_properties = carried_props;
        self.send_to(ServerToClient::Carry (self.objects[carrier].get_id(), self.objects[carried].get_id()), self.objects[carried].get_banner());
        // NOTE: If this doesn't work because of the borrow checker mad at having 2 (3???) mutable references to self.objects, just use copying on the ExposedProperties!
        // Since carrying is a relatively rare operation, the wastefulness is not significant.
    }

    pub fn player_died(&mut self, player : usize, was_rtf : bool) { // player banner, to be #exact
        for i in 0..self.teams.len() {
            if let Some(index) = self.teams[i].members.iter().position(|value| *value == player) {
                self.teams[i].members.swap_remove(index);
            }
        }
        self.living_players -= 1;
        for i in 0..self.teams.len() {
            if self.teams[i].members.len() == self.living_players as usize {
                self.broadcast(ServerToClient::End (self.teams[i].banner_id as u32));
                return;
            }
        }
        if !was_rtf {
            self.isnt_rtf -= 1;
        }
        self.broadcast_tx.send(ClientCommand::SomeoneDied (player)).unwrap();
        self.broadcast_tx.send(ClientCommand::RoleCall).unwrap();
        println!("Player died. Living players: {}, connected clients: {}", self.living_players, self.clients_connected);
    }

    fn deal_with_one_object(&mut self, x : usize, y : usize) {
        if x == y {
            println!("SYSTEM BROKE BECAUSE X EQUALS Y! CHECK YOUR MATH!");
        }
        if self.objects[x].exposed_properties.carrier_properties.is_carried || self.objects[y].exposed_properties.carrier_properties.is_carried {
            return; // Never do any kind of collisions on carried objects.
        }
        if !self.objects[x].get_does_collide(self.objects[y].identify()) && !self.objects[y].get_does_collide(self.objects[x].identify()) {
            return; // they can't possibly interact with each other so there's no reason to do any physics checks at all
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
                if self.objects[x].dead() && (self.objects[y].get_banner() != self.objects[x].get_banner() || self.objects[x].identify() == 'g') {
                    /*let killah = self.get_client_by_banner(self.objects[y].get_banner()).await;
                    if killah.is_some() {
                        let amount = self.objects[x].capture().await as i32;
                        killah.unwrap().lock().await.collect(amount).await;
                    }*/
                    if self.objects[x].does_give_score() {
                        self.broadcast_tx.send(ClientCommand::ScoreTo (self.objects[y].get_banner(), self.objects[x].capture() as i32)).expect("Broadcast failed");
                    }
                    if self.objects[x].does_grant_a2a() {
                        self.broadcast_tx.send(ClientCommand::GrantA2A (self.objects[y].get_banner())).expect("Broadcast failed part 2");
                    }
                }
                is_collide = true;
            }
            if self.objects[y].get_does_collide(self.objects[x].identify()) {
                let dmg = self.objects[x].get_collision_info().damage;
                self.objects[y].damage(dmg);
                if self.objects[y].dead() && (self.objects[y].get_banner() != self.objects[x].get_banner() || self.objects[y].identify() == 'g') {
                    /*let killah = self.get_client_by_banner(self.objects[x].get_banner()).await;
                    if killah.is_some() {
                        let amount = self.objects[y].capture().await as i32;
                        killah.unwrap().lock().await.collect(amount).await;
                    }*/
                    if self.objects[y].does_give_score() {
                        self.broadcast_tx.send(ClientCommand::ScoreTo (self.objects[x].get_banner(), self.objects[y].capture() as i32)).expect("Broadcast failed");
                    }
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
                    let objects = &mut self.objects as *mut Vec<GamePieceBase>;
                    let obj = &mut (*objects)[i];
                    obj.die(self);
                    for subscriber in 0..obj.death_subscriptions.len() {
                        let thing = self.obj_lookup(obj.death_subscriptions[subscriber]);
                        match thing {
                            Some(thing) => {
                                (*(&mut self.objects as *mut Vec<GamePieceBase>))[thing].on_subscribed_death(obj, self);
                            }
                            None => {}
                        }
                    }
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
                //println!("Deleting a {} with id {}", self.objects[i].identify(), self.objects[i].get_id());
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
        if self.self_test { // run update routines ONLY, ignoring clients.
            self.deal_with_objects();
            return;
        }
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
                        if team.members.len() == self.living_players as usize { // If one team holds all the players
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
            /*if !self.is_io {
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
            }*/
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
            for _ in 0..std::cmp::min(((self.gamesize * self.gamesize) / 1000000.0) as u32, 300) { // One per 1,000,000 square pixels, or 200, whichever is lower.
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

    fn send_to(&self, message : ServerToClient, banner : usize) {
        self.broadcast_tx.send(ClientCommand::SendTo (message, banner)).expect("Broadcast Failed");
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
            self.broadcast_tx.send(ClientCommand::AttachToBanner (piece.get_id(), banner.unwrap(), if self.costs { piece.cost() } else { 0 })).expect("Broadcast FAILED!");
        }
        self.broadcast(piece.get_new_message());
        let ret = piece.get_id();
        self.objects.push(piece);
        ret
    }

    fn authenticate(&self, password : String, spectator : bool) -> AuthState {
        if self.admin_password == password {
            return AuthState::God;
        }
        // God can't be on a team. Is this a profound philosophical metaphor???
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
        self.isnt_rtf = 0;
        self.living_players = 0;
        self.clients_connected = 0;
        self.set_mode(GameMode::Waiting);
        self.clear_banners();
        self.load_config();
    }

    fn clear_banners(&mut self) {
        println!("Clearing banners...");
        while self.banners.len() > 1 { 
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
            members: vec![]
        });
    }
}


impl Client {
    fn new(socket : WebSocketClientStream, commandah : tokio::sync::mpsc::Sender<ServerCommand>) -> Self {
        Self {
            socket,
            is_superuser : false,
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
            kys: false,
            a2a: 0,
            walls_cap : 2,
            walls_remaining : 4, // you get a bonus on turn 1
            is_ready : false
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
                ClientToServer::ReadyState (v) => {
                    if v != self.is_ready {
                        self.is_ready = v;
                        self.commandah.send(ServerCommand::ReadyState (self.is_ready)).await.unwrap();
                    }
                }
                ClientToServer::Place (x, y, tp, variant) => {
                    if self.game_cmode == GameMode::Play && tp != b'c' && !self.is_superuser { // if it is trying to place an object, but it isn't strat mode or waiting mode and it isn't placing a castle
                        // originally, this retaliated, but now it just refuses. the retaliation was a problem.
                        println!("ATTEMPT TO PLACE IN PLAY MODE");
                        return; // can't place things if it ain't strategy. Also this is poison.
                    }
                    let fire_banner = if self.is_superuser { None } else { Some(self.banner) };
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
                                self.commandah.send(ServerCommand::Place (PlaceCommand::SimplePlace (x, y, fire_banner, b'w'))).await.unwrap();
                                if !self.is_superuser {
                                    self.walls_remaining -= 1;
                                }
                            }
                        },
                        b'K' => {
                            match variant {
                                0 => {
                                    self.commandah.send(ServerCommand::Place (PlaceCommand::SimplePlace (x, y, fire_banner, b'K'))).await.unwrap();
                                },
                                _ => {
                                    self.commandah.send(ServerCommand::Place (PlaceCommand::CarrierVariant (x, y, fire_banner, variant))).await.unwrap();
                                }
                            }
                        },
                        b'F' => {
                            match self.m_castle {
                                Some(cid) => {
                                    self.commandah.send(ServerCommand::Place (PlaceCommand::Fort (x, y, fire_banner, cid))).await.unwrap();
                                }
                                None => {}
                            }
                        },
                        _ => {
                            self.commandah.send(ServerCommand::Place (PlaceCommand::SimplePlace (x, y, fire_banner, tp))).await.unwrap();
                        }
                    }
                },
                ClientToServer::Cost (amount) => {
                    let amount = amount.abs();
                    println!("Costing an amount of money equal to {amount} coins");
                    self.collect(-amount).await;
                },
                ClientToServer::Move (id, x, y, a) => {
                    self.commandah.send(ServerCommand::Move (self.banner, id, x, y, a, self.is_superuser)).await.unwrap();
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
                ClientToServer::GodDelete (id) => {
                    if self.is_superuser {
                        self.commandah.send(ServerCommand::RejectObject (id)).await.unwrap();
                    }
                },
                ClientToServer::GodReset => {
                    if self.is_superuser {
                        self.commandah.send(ServerCommand::Reset).await.unwrap();
                    }
                }
                ClientToServer::GodDisconnect (cli) => {
                    if self.is_superuser {
                        self.commandah.send(ServerCommand::GodDisconnect (cli as usize)).await.unwrap();
                    }
                }
                ClientToServer::GodFlip => {
                    if self.is_superuser {
                        self.commandah.send(ServerCommand::Flip).await.unwrap();
                    }
                }
                ClientToServer::GodNuke (n) => {
                    if self.is_superuser {
                        self.commandah.send(ServerCommand::Nuke (n as usize)).await.unwrap();
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
                                            AuthState::God => {
                                                self.send_protocol_message(ServerToClient::Welcome).await;
                                                self.send_protocol_message(ServerToClient::YouAreGod).await;
                                                self.is_authorized = true;
                                                self.is_superuser = true;
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

    /*fn is_alive(&self, server : tokio::sync::MutexGuard<'_, Server>) -> bool {
        if self.m_castle.is_some() {
            if server.obj_lookup(self.m_castle.unwrap()).is_some() {
                return true;
            }
        }
        false
    }*/
}


async fn got_client(client : WebSocketClientStream, broadcaster : tokio::sync::broadcast::Sender<ClientCommand>, commandset : tokio::sync::mpsc::Sender<ServerCommand>){
    commandset.send(ServerCommand::Connect).await.unwrap();
    let mut receiver = broadcaster.subscribe();
    let mut moi = Client::new(client, commandset);
    let mut dead = false; // TODO: move this into Client
    /*
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
                    Ok (ClientCommand::Close (whom)) => {
                        if moi.banner == whom {
                            break 'cliloop;
                        }
                    },
                    Ok (ClientCommand::Tick (counter, mode)) => {
                        moi.game_cmode = mode;
                        if mode == GameMode::Play { // if it's play mode
                            moi.walls_remaining = moi.walls_cap; // it can't use 'em until next turn, ofc
                            moi.is_ready = false;
                        }
                        //let mut schlock = server.lock().await;
                        //schlock.winning_banner = moi.banner;
                        moi.send_protocol_message(ServerToClient::Tick (counter, match mode {
                            GameMode::Play => 0, 
                            GameMode::Strategy => 1,
                            GameMode::Waiting => 2
                        })).await;
                        /*for obj in &schlock.objects {
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
                        }*/
                    },
                    Ok (ClientCommand::Christmas) => {
                        moi.collect(10000).await;
                    },
                    Ok (ClientCommand::RoleCall) => {
                        if !dead {
                            match moi.commandah.send(ServerCommand::WinningBanner (moi.banner, moi.mode == ClientMode::RealTimeFighter)).await {
                                _ => {}
                            }; // ignore the error; eventually, we may handle it. The error here is because we're trying to send winner information to a disconnected client.
                        }
                    },
                    Ok (ClientCommand::SomeoneDied (banner)) => {
                        if banner == moi.banner {
                            moi.send_protocol_message(ServerToClient::YouLose).await;
                            dead = true;
                        }
                    }
                    Ok (ClientCommand::HealthStream (id, value)) => {
                        moi.send_protocol_message(ServerToClient::HealthUpdate (id, value)).await;
                    },
                    Ok (ClientCommand::Send (message)) => {
                        moi.send_protocol_message(message).await;
                    },
                    Ok (ClientCommand::SendTo (message, banner)) => {
                        if moi.banner == banner {
                            moi.send_protocol_message(message).await;
                        }
                    }
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
                    Ok (ClientCommand::AttachToBanner (id, banner, price)) => {
                        if banner == moi.banner {
                            /*let mut reject : bool = false;
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
                            }*/
                            if moi.score >= price || moi.is_superuser {
                                moi.collect(-price).await;
                                moi.send_protocol_message(ServerToClient::Add (id)).await;
                            }
                            else {
                                println!("REJECTING OBJECT WE CAN'T AFFORD!");
                                moi.commandah.send(ServerCommand::RejectObject (id)).await.expect("Failed!");
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
    //moi.commandah.send(ServerCommand::LivePlayerDec (moi.team, moi.mode)).await.expect("Broadcast failed");
    /*let mut serverlock = server.lock().await;
    if moi.m_castle.is_some() && serverlock.obj_lookup(moi.m_castle.unwrap()).is_some() {
        moi.commandah.send(ServerCommand::LivePlayerDec (moi.team, moi.mode)).await.expect("Broadcast failed");
    }*/
    moi.close();
    /*if moi.team.is_some(){ // Remove us from the team
        let mut i = 0;
        while i < serverlock.teams[moi.team.unwrap()].members.len() {
            if serverlock.teams[moi.team.unwrap()].members[i] == moi.banner {
                serverlock.teams[moi.team.unwrap()].members.remove(i);
                break;
            }
            i += 1;
        }
    }
    if moi.is_authorized {
        serverlock.authenticateds -= 1;
    }
    if serverlock.is_io || serverlock.mode == GameMode::Waiting {
        serverlock.clear_of_banner(moi.banner);
    }*/
    if moi.is_ready {
        moi.commandah.send(ServerCommand::ReadyState (false)).await.unwrap();
    }
    moi.commandah.send(ServerCommand::Disconnect (moi.mode, moi.banner, moi.m_castle)).await.unwrap();
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
    A2A (u32, u32, usize), // gunner, target, banner
    CarrierVariant (f32, f32, Option<usize>, u32) // x, y, banner, variant
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
    SelfTest,
    Start,
    Flip,
    Christmas,
    IoModeToggle,
    PasswordlessToggle,
    Autonomous (u32, u32, u32),
    TeamNew (String, String),
    Connect,
    Disconnect (ClientMode, usize, Option<u32>), // mode of the disconnecting client, banner of disconnecting client, castle of the disconnecting client if applicable.
    Broadcast (String),
    RejectObject (u32),
    PrintBanners,
    Nuke (usize),
    Reset,
    Place (PlaceCommand),
    Move (usize, u32, f32, f32, f32, bool), // banner, id, x, y, a, is_superuser
    PilotRTF (u32, bool, bool, bool, bool, bool),
    Chat (usize, String, u8, Option<usize>),
    UpgradeNextTier (u32, String),
    BeginConnection (String, String, String, tokio::sync::mpsc::Sender<InitialSetupCommand>), // password, banner, mode, outgoing pipe. god i've got to clean this up. vomiting face.
    WinningBanner (usize, bool), // report a banner that is alive and whether or not the player is an rtf. the server will do some routines.
    ReadyState (bool),
    GodDisconnect (usize) // disconnect a player
}

const WORDLIST : [&str; 10] = ["Robust", "Nancy", "Sovereign", "Green", "Tailor", "Water", "Freebase", "Neon", "Morlock", "Rastafari"];


#[tokio::main]
async fn main(){
    let args: Vec<String> = std::env::args().collect();
    let mut rng = rand::thread_rng();
    use rand::prelude::SliceRandom;
    let (broadcast_tx, _rx) = tokio::sync::broadcast::channel(128); // Give _rx a name because we want it to live to the end of this function; if it doesn't, the tx will be invalidated. or something.
    let mut admin_password = String::new();
    for x in 0..4 {
        admin_password += WORDLIST.choose(&mut rng).unwrap();
        if x < 3 {
            admin_password += " ";
        }
    }
    let mut server = Server {
        self_test           : false,
        mode                : GameMode::Waiting,
        admin_password,
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
        isnt_rtf            : 0,
        times               : (40.0, 20.0),
        clients_connected   : 0,
        is_headless         : false,
        permit_npcs         : true,
        port                : 0,
        sql                 : "default.db".to_string(),
        worldzone_count     : 1,
        zones               : Vec::new(),
        vvlm                : false,
        readies             : 0
    };
    //rx.close().await;
    server.load_config();
    let port = server.port;
    let headless = server.is_headless;
    println!("Started server with password {}, terrain seed {}. The admin password is {}.", server.password, server.terrain_seed, server.admin_password);
    let (commandset, mut commandget) = tokio::sync::mpsc::channel(32); // fancy number
    let commandset_clone = commandset.clone();
    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis((1000.0/FPS) as u64));
        let connection = sqlite::open(server.sql.clone()).unwrap();
        let init_query = "CREATE TABLE IF NOT EXISTS logins (banner TEXT, password TEXT, highscore INTEGER, wins INTEGER, losses INTEGER);CREATE TABLE IF NOT EXISTS teams_records (teamname TEXT, wins INTEGER, losses INTEGER);";
        connection.execute(init_query).unwrap();
        loop {
            select! {
                _ = interval.tick() => {
                    use tokio::time::Instant;
                    let start = Instant::now();
                    server.mainloop();
                    if start.elapsed() > tokio::time::Duration::from_millis((1000.0/FPS) as u64) {
                        if server.self_test {
                            println!("Failure at {} objects", server.objects.len());
                            let obj = server.objects.len() - 1;
                            server.objects[obj].damage(100.0);
                            server.objects[obj].damage(100.0);
                        }
                        else {
                            println!("LOOP OVERRUN!");
                        }
                    }
                    else if server.self_test {
                        server.place_random_rubble();
                        if server.objects.len() % 10 == 0 {
                            println!("reached {} objects", server.objects.len());
                        }
                    }
                },
                command = commandget.recv() => {
                    //println!("honk");
                    match command {
                        Some (ServerCommand::GodDisconnect (n)) => {
                            server.broadcast_tx.send(ClientCommand::Close (n)).unwrap();
                        }
                        Some (ServerCommand::Start) => {
                            server.start();
                        },
                        Some (ServerCommand::ReadyState (v)) => {
                            if v {
                                server.readies += 1;
                            }
                            else {
                                server.readies -= 1;
                            }
                            if server.readies >= server.living_players {
                                if server.mode != GameMode::Play {
                                    server.readies = 0;
                                    server.set_mode(GameMode::Play);
                                }
                            }
                        }
                        Some (ServerCommand::Christmas) => {
                            server.broadcast_tx.send(ClientCommand::Christmas).unwrap();
                        }
                        Some (ServerCommand::RejectObject (id)) => {
                            server.delete_obj(id);
                        },
                        Some (ServerCommand::Flip) => {
                            server.flip();
                        },
                        Some (ServerCommand::TeamNew (name, password)) => {
                            server.new_team(name, password);
                        },
                        Some (ServerCommand::Autonomous (min_players, max_players, auto_timeout)) => {
                            server.autonomous = Some((min_players, max_players, auto_timeout, auto_timeout));
                        },
                        Some (ServerCommand::Move (banner, id, x, y, a, superuser)) => {
                            for object in &mut server.objects {
                                if object.get_id() == id && (object.get_banner() == banner || superuser) {
                                    object.exposed_properties.goal_x = x;
                                    object.exposed_properties.goal_y = y;
                                    object.exposed_properties.goal_a = a;
                                }
                            }
                        },
                        Some (ServerCommand::UpgradeNextTier (item, upgrade)) => {
                            server.upgrade_next_tier(item, upgrade);
                        },
                        Some (ServerCommand::WinningBanner (banner, _is_rtf)) => {
                            if !server.is_io && server.living_players == 1 {
                                server.broadcast(ServerToClient::End (banner as u32));
                                println!("{} won the game!", banner);
                            }
                        },
                        Some (ServerCommand::PilotRTF (id, fire, left, right, airbrake, shoot)) => {
                            match server.obj_lookup(id) {
                                Some (index) => {
                                    let mut thrust = 0.0;
                                    let mut resistance = 1.0;
                                    let mut angle_thrust = 0.0;
                                    let is_better_turns = server.objects[index].upgrades.contains(&"f3".to_string());
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
                                    server.objects[index].exposed_properties.shooter_properties.suppress = !shoot;
                                    let thrust = Vector2::new_from_manda(thrust, server.objects[index].exposed_properties.physics.angle() - PI/2.0);
                                    server.objects[index].exposed_properties.physics.velocity += thrust;
                                    server.objects[index].exposed_properties.physics.velocity *= resistance;
                                    server.objects[index].exposed_properties.physics.angle_v += angle_thrust;
                                    server.objects[index].exposed_properties.physics.angle_v *= if is_better_turns { 0.8 } else { 0.9 };
                                    server.objects[index].exposed_properties.physics.angle_v *= resistance;
                                }
                                None => {}
                            }
                        },
                        Some (ServerCommand::BeginConnection (password, banner, mode, transmit)) => {
                            let banner_id = server.banner_add(banner);
                            if server.new_user_can_join() {
                                let thing = server.authenticate(password, mode == "spectator");
                                match thing {
                                    AuthState::Error => {
                                        println!("Authentication error");

                                    },
                                    AuthState::God => {
                                        println!("========== God joined ==========");
                                        transmit.send(InitialSetupCommand::Joined (AuthState::God)).await.unwrap();
                                    },
                                    AuthState::Single => {
                                        println!("Single player joined");
                                        transmit.send(InitialSetupCommand::Joined (AuthState::Single)).await.unwrap();
                                    },
                                    AuthState::Team (teamid, _) => {
                                        println!("Player {} joined to team {}", server.banners[banner_id], server.banners[server.teams[teamid].banner_id]);
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
                            for object in &server.objects {
                                transmit.send(InitialSetupCommand::Message (object.get_new_message())).await.unwrap();
                                for upg in &object.upgrades {
                                    transmit.send(InitialSetupCommand::Message (ServerToClient::UpgradeThing(object.exposed_properties.id, upg.clone()))).await.unwrap();
                                }
                            }
                            for i in 0..server.banners.len() {
                                transmit.send(InitialSetupCommand::Message (ServerToClient::BannerAdd(i as u32, server.banners[i].clone()))).await.unwrap();
                            }
                            for i in 0..server.teams.len() {
                                for j in 0..server.teams[i].members.len() {
                                    transmit.send(InitialSetupCommand::Message (ServerToClient::BannerAddToTeam(server.teams[i].members[j] as u32, server.teams[i].banner_id as u32))).await.unwrap();
                                }
                            }
                            server.authenticateds += 1;
                            transmit.send(InitialSetupCommand::Metadata (server.gamesize, banner_id)).await.unwrap();
                            transmit.send(InitialSetupCommand::Finished).await.unwrap();
                        },
                        Some (ServerCommand::IoModeToggle) => {
                            server.is_io = !server.is_io;
                            println!("Set io mode to {}", server.is_io);
                        },
                        Some (ServerCommand::Disconnect (mode, banner, castle)) => {
                            if castle.is_some() && server.obj_lookup(castle.unwrap()).is_some() {
                                server.player_died(banner, mode == ClientMode::RealTimeFighter);
                                println!("yuh");
                            }
                            server.clear_of_banner(banner);
                            if server.clients_connected == 0 {
                                server.reset();
                            }
                            else {
                                server.clients_connected -= 1;
                            }
                        },
                        Some (ServerCommand::Connect) => {
                            server.clients_connected += 1;
                        },
                        Some (ServerCommand::Broadcast (message)) => {
                            server.chat(message, 0, 6, None);
                        },
                        Some (ServerCommand::Chat (banner, message, priority, to_whom)) => {
                            println!("{} says {}", server.banners[banner], message);
                            server.chat(message, banner, priority, to_whom);
                        }
                        Some (ServerCommand::PasswordlessToggle) => {
                            server.passwordless = !server.passwordless;
                            server.broadcast(ServerToClient::SetPasswordless (server.passwordless));
                            println!("Set passwordless mode to {}", server.passwordless);
                        },
                        /*Some (ServerCommand::LivePlayerInc (team, mode)) => {
                            if mode != ClientMode::RealTimeFighter {
                                println!("{:?} isn't an rtf", mode);
                                server.isnt_rtf += 1;
                            }
                            server.living_players += 1;
                            if team.is_some() {
                                server.teams[team.unwrap()].live_count += 1;
                            }
                            println!("New live player. Living players: {}", server.living_players);
                        },
                        Some (ServerCommand::LivePlayerDec (team, mode)) => {
                            if mode != ClientMode::RealTimeFighter {
                                server.isnt_rtf -= 1;
                            }
                            server.living_players -= 1;
                            if team.is_some() {
                                server.teams[team.unwrap()].live_count -= 1;
                            }
                            println!("Player died. Living players: {}", server.living_players);
                        },*/
                        Some (ServerCommand::PrintBanners) => {
                            println!("Current banners are,");
                            for banner in 0..server.banners.len() {
                                println!("{}: {}", banner, server.banners[banner]);
                            }
                        }
                        Some (ServerCommand::Nuke (banner)) => {
                            for object in 0..server.objects.len() {
                                if server.objects[object].get_banner() == banner {
                                    let x = server.objects[object].exposed_properties.physics.cx();
                                    let y = server.objects[object].exposed_properties.physics.cy();
                                    server.place_nuke(x, y, 0.0, None);
                                }
                            }
                        },
                        Some (ServerCommand::Reset) => {
                            server.reset()
                        },
                        Some (ServerCommand::Place (PlaceCommand::SimplePlace (x, y, banner, tp))) => {
                            match tp {
                                b'f' => {
                                    server.place_basic_fighter(x, y, 0.0, banner);
                                },
                                b'm' => {
                                    server.place_mls(x, y, 0.0, banner);
                                },
                                b'a' => {
                                    server.place_antirtf_missile(x, y, 0.0, banner);
                                },
                                b'K' => {
                                    server.place_carrier(x, y, 0.0, banner);
                                },
                                b't' => {
                                    server.place_tie_fighter(x, y, 0.0, banner);
                                },
                                b's' => {
                                    server.place_sniper(x, y, 0.0, banner);
                                },
                                b'h' => {
                                    server.place_missile(x, y, 0.0, banner);
                                },
                                b'T' => {
                                    server.place_turret(x, y, 0.0, banner);
                                },
                                b'n' => {
                                    server.place_nuke(x, y, 0.0, banner);
                                },
                                b'w' => {
                                    server.place_wall(x, y, banner);
                                },
                                b'S' => {
                                    server.place_seed(x, y, banner);
                                },
                                b'G' => {
                                    server.place_green_thumb(x, y, banner);
                                },
                                b'g' => {
                                    server.place_gold_bar(x, y, banner);
                                }
                                _ => {
                                    println!("The client attempted to place an object with invalid type {}", tp);
                                }
                            }
                        }
                        Some (ServerCommand::Place (PlaceCommand::CarrierVariant (x, y, banner, variant))) => {
                            let carrier = server.place_carrier(x, y, 0.0, banner);
                            let carrier = server.obj_lookup(carrier).unwrap();
                            /*match variant {
                                1 => {
                                    let turret = server.place_turret(x, y, 0.0, banner);
                                    server.into_berth(carrier, turret, 0);
                                    let turret = server.place_turret(x, y, 0.0, banner);
                                    server.into_berth(carrier, turret, 1);
                                    let turret = server.place_turret(x, y, 0.0, banner);
                                    server.into_berth(carrier, turret, 8);
                                    let turret = server.place_turret(x, y, 0.0, banner);
                                    server.into_berth(carrier, turret, 9);

                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 2);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 3);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 6);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 7);

                                    let nuke = server.place_nuke(x, y, 0.0, banner);
                                    server.into_berth(carrier, nuke, 4);
                                    let nuke = server.place_nuke(x, y, 0.0, banner);
                                    server.into_berth(carrier, nuke, 5);
                                },
                                2 => {
                                    let turret = server.place_turret(x, y, 0.0, banner);
                                    server.into_berth(carrier, turret, 0);
                                    let turret = server.place_turret(x, y, 0.0, banner);
                                    server.into_berth(carrier, turret, 1);

                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 4);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 5);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 6);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 7);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 8);
                                    let hyper = server.place_missile(x, y, 0.0, banner);
                                    server.into_berth(carrier, hyper, 9);

                                    let nuke = server.place_nuke(x, y, 0.0, banner);
                                    server.into_berth(carrier, nuke, 2);
                                    let nuke = server.place_nuke(x, y, 0.0, banner);
                                    server.into_berth(carrier, nuke, 3);
                                },
                                3 => {
                                    for i in 0..10 {
                                        let thing = if i % 2 == 0 { server.place_missile(x, y, 0.0, banner) } else { server.place_turret(x, y, 0.0, banner) };
                                        server.into_berth(carrier, thing, i);
                                    }
                                },
                                4 => {
                                    for i in 0..10 {
                                        let thing = server.place_missile(x, y, 0.0, banner);
                                        server.into_berth(carrier, thing, i);
                                    }
                                },
                                5 => {
                                    for i in 0..4 {
                                        let tie = server.place_tie_fighter(x, y, 0.0, banner);
                                        server.into_berth(carrier, tie, i);
                                    }
                                    for i in 4..10 {
                                        let hyper = server.place_missile(x, y, 0.0, banner);
                                        server.into_berth(carrier, hyper, i);
                                    }
                                },
                                6 => {
                                    for i in 0..10 {
                                        let thing = server.place_tie_fighter(x, y, 0.0, banner);
                                        server.into_berth(carrier, thing, i);
                                    }
                                },
                                _ => {
                                    println!("Unrecognized carrier variant {}", variant);
                                }
                            }*/
                            // bitbanged
                            for i in 0..10_usize { // 10 berths
                                let bit = 9_u32.pow(i as u32);
                                let word = (variant / bit) % 9; // it's complex math. don't worry your sweet wittle head about it.
                                let item = match word {
                                    1 => {
                                        server.place_missile(x, y, 0.0, banner)
                                    },
                                    2 => {
                                        server.place_basic_fighter(x, y, 0.0, banner)
                                    },
                                    3 => {
                                        server.place_tie_fighter(x, y, 0.0, banner)
                                    },
                                    4 => {
                                        server.place_sniper(x, y, 0.0, banner)
                                    },
                                    5 => {
                                        server.place_nuke(x, y, 0.0, banner)
                                    },
                                    6 => {
                                        server.place_turret(x, y, 0.0, banner)
                                    },
                                    7 => {
                                        server.place_mls(x, y, 0.0, banner)
                                    },
                                    8 => {
                                        server.place_gold_bar(x, y, banner)
                                    }
                                    _ => {
                                        0
                                    }
                                };
                                if item != 0 {
                                    server.into_berth(carrier, item, i);
                                }
                            }
                        }
                        Some (ServerCommand::Place (PlaceCommand::Fort (x, y, banner, target))) => {
                            match server.obj_lookup(target) {
                                Some(index) => {
                                    let fort = server.place_fort(x, y, 0.0, banner);
                                    server.objects[index].add_fort(fort);
                                }
                                None => {}
                            }
                        }
                        Some (ServerCommand::SelfTest) => {
                            server.self_test = true;
                        }
                        Some (ServerCommand::Place (PlaceCommand::Castle (x, y, mode, banner, team))) => {
                            if server.mode != GameMode::Waiting && !server.is_io {
                                continue;
                            }
                            server.costs = false;
                            let castle = server.place_castle(x, y, mode == ClientMode::RealTimeFighter, Some(banner));
                            server.broadcast_tx.send(ClientCommand::SetCastle (banner, castle)).unwrap();
                            match mode {
                                ClientMode::Normal => {
                                    server.place_basic_fighter(x - 200.0, y, PI, Some(banner));
                                    server.place_basic_fighter(x + 200.0, y, 0.0, Some(banner));
                                    server.place_basic_fighter(x, y - 200.0, 0.0, Some(banner));
                                    server.place_basic_fighter(x, y + 200.0, 0.0, Some(banner));
                                    server.broadcast_tx.send(ClientCommand::ScoreTo (banner, 100)).unwrap();
                                },
                                ClientMode::RealTimeFighter => {
                                    server.place_basic_fighter(x - 100.0, y, PI, Some(banner));
                                    server.place_basic_fighter(x + 100.0, y, 0.0, Some(banner));
                                    server.broadcast_tx.send(ClientCommand::GrantA2A (banner)).unwrap();
                                },
                                ClientMode::Defense => {
                                    server.place_basic_fighter(x - 200.0, y, PI, Some(banner));
                                    server.place_basic_fighter(x + 200.0, y, 0.0, Some(banner));
                                    server.place_turret(x, y - 200.0, 0.0, Some(banner));
                                    server.place_turret(x, y + 200.0, 0.0, Some(banner));
                                    server.broadcast_tx.send(ClientCommand::ScoreTo (banner, 25)).unwrap();
                                },
                                _ => {

                                }
                            }
                            // shamelessly copy/pasted from LivePlayerInc. clean up when the dust settles!
                            server.costs = true;
                            if mode != ClientMode::RealTimeFighter {
                                println!("{:?} isn't an rtf", mode);
                                server.isnt_rtf += 1;
                            }
                            server.living_players += 1;
                            if team.is_some() {
                                server.teams[team.unwrap()].members.push(banner);
                                server.broadcast(ServerToClient::BannerAddToTeam (banner as u32, server.teams[team.unwrap()].banner_id as u32));
                            }
                            println!("New live player. Living players: {}", server.living_players);
                        },
                        Some (ServerCommand::Place (PlaceCommand::A2A (castle, target, banner))) => {
                            let target_i = match server.obj_lookup(target) { Some(i) => i, None => continue };
                            let castle_i = match server.obj_lookup(castle) { Some(i) => i, None => continue };
                            let obj_vec = server.objects[target_i].exposed_properties.physics.vector_position();
                            if (server.objects[castle_i].exposed_properties.physics.vector_position() - obj_vec).magnitude() < 1500.0 {
                                let off_ang = functions::coterminal(server.objects[castle_i].exposed_properties.physics.angle() - (server.objects[castle_i].exposed_properties.physics.vector_position() - obj_vec).angle(), PI * 2.0);
                                let pos = server.objects[castle_i].exposed_properties.physics.vector_position() + Vector2::new_from_manda(if off_ang > PI { 50.0 } else { -50.0 }, server.objects[castle_i].exposed_properties.physics.angle());
                                let launchangle = server.objects[castle_i].exposed_properties.physics.angle() - PI/2.0; // rust requires this to be explicit because of the dumbass borrow checker
                                let a2a_id = server.place_air2air(pos.x, pos.y, launchangle, target, Some(banner));
                                let a2a_i = server.obj_lookup(a2a_id).unwrap(); // it's certain to exist
                                server.objects[a2a_i].exposed_properties.physics.velocity = server.objects[castle_i].exposed_properties.physics.velocity;
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
        tokio::task::spawn(cli(commandset));
        //std::thread::spawn(|| {
        //    cli(commandset).await;
        //});
    }

    let mut websocket_server = WebSocketServer::new(port, "MMOSG".to_string()).await;
    println!("made it here");
    loop {
        let client = websocket_server.accept::<ClientToServer, ServerToClient>().await;
        tokio::task::spawn(got_client(client, broadcast_tx.clone(), commandset_clone.clone()));
    }
}


async fn cli(commandset : tokio::sync::mpsc::Sender<ServerCommand>) {
    use tokio::io::AsyncBufReadExt;
    let buffer = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = buffer.lines();
    loop {
        let command = match lines.next_line().await { Ok(Some(line)) => line, Ok(None) => continue, Err(_) => continue };
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
                ServerCommand::TeamNew (name, password)
            },
            "toggle iomode" => {
                ServerCommand::IoModeToggle
            },
            "santa" => {
                ServerCommand::Christmas
            }
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
            "selftest" => {
                ServerCommand::SelfTest
            }
            _ => {
                println!("Invalid command.");
                continue;
            }
        };
        commandset.send(to_send).await.expect("OOOOOOPS");
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
