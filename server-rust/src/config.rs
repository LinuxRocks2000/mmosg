use serde::{Deserialize, Serialize};
use crate::Server;
use crate::input;

#[derive(Serialize, Deserialize)]
struct ObjectDef {
    x       : f32,
    y       : f32,
    w       : f32,
    h       : f32,
    a       : Option<f32>
}


#[derive(Serialize, Deserialize)]
pub struct AutonomousDef {
    min_players : u32,
    max_players : u32,
    timeout     : u32
}


#[derive(Serialize, Deserialize)]
pub struct TeamDef {
    name: String,
    password: String
}


#[derive(Serialize, Deserialize)]
pub struct ExtObjectDef {
    t : String,
    x : f32,
    y : f32,
    effect_radius : Option<f32> // for any ext type with an effect radius
}


#[derive(Serialize, Deserialize)]
struct ServerConfigFile {
    password        : Option<String>,
    world_size      : f32,
    io_mode         : Option<bool>,
    prompt_password : Option<bool>,
    map             : Vec<ObjectDef>,
    autonomous      : Option<AutonomousDef>,
    teams           : Option<Vec<TeamDef>>,
    strat_secs      : Option<f32>,
    play_secs       : Option<f32>,
    headless        : Option<bool>,
    permit_npcs     : Option<bool>,
    port            : Option<u16>,
    database        : Option<String>,
    map_anchor      : Option<String>,
    zones           : Option<usize>,
    ext             : Option<Vec<ExtObjectDef>>
}

pub struct Config {
    json : ServerConfigFile
}

impl Config {
    pub fn new(file : &str) -> Self {
        use std::fs;
        println!("Loading configuration from {}", file);
        let json_reader = fs::File::open(file).expect("Error reading config file");
        let json : ServerConfigFile = serde_json::from_reader(json_reader).expect("Error parsing JSON!");
        Self {
            json
        }
    }

    pub fn load_into(&self, server : &mut Server) {
        server.gamesize = self.json.world_size;
        if self.json.world_size > 50000.0 {
            server.vvlm = true;
        }
        server.port = match self.json.port {
            Some(port) => port,
            None => 3000
        };
        if self.json.headless.is_some() {
            server.is_headless = self.json.headless.unwrap();
        }
        if self.json.password.is_some() {
            server.passwordless = false;
            server.password = self.json.password.as_ref().unwrap().clone();
        }
        else {
            server.passwordless = true;
        }
        if self.json.prompt_password.is_some() && self.json.prompt_password.unwrap() {
            server.passwordless = false;
            server.password = input("Game password: ");
        }
        else {
            server.passwordless = true;
        }
        if self.json.teams.is_some() {
            server.passwordless = false;
            for team in self.json.teams.as_ref().unwrap() {
                server.new_team(team.name.clone(), team.password.clone());
            }
        }
        let is_tl = match &self.json.map_anchor {
            Some (anchor) => anchor == "topleft",
            None => false
        };
        for def in &self.json.map {
            let mut x = def.x;
            let mut y = def.y;
            let a = match def.a {
                Some(a) => a * std::f32::consts::PI/180.0,
                None => 0.0
            };
            if is_tl {
                x += def.w/2.0; // at this point in existence, it isn't rotated - after we've converted to cx,cy, it'll be rotated by the place_block call.
                y += def.h/2.0;
            }
            server.place_block(x, y, a, def.w, def.h);
        }
        match &self.json.ext {
            Some(ext) => {
                for def in ext {
                    match def.t.as_str() {
                        "nexus" => {
                            server.place_nexus(def.x, def.y, def.effect_radius.unwrap());
                        },
                        &_ => {
                            panic!("Bad type in the config file!");
                        }
                    }
                }
            },
            _ => {}
        }
        if self.json.io_mode.is_some() {
            server.is_io = self.json.io_mode.unwrap();
        }
        if self.json.permit_npcs.is_some() {
            server.permit_npcs = self.json.permit_npcs.unwrap();
        }
        match &self.json.autonomous {
            Some(auto) => {
                server.autonomous = Some((auto.min_players, auto.max_players, auto.timeout, auto.timeout));
            },
            None => {}
        }
        match self.json.strat_secs {
            Some(time) => {
                server.times.0 = time;
            },
            _ => {}
        }
        match self.json.play_secs {
            Some(time) => {
                server.times.1 = time;
            },
            _ => {}
        }
        match &self.json.database {
            Some(database_fname) => {
                server.sql = database_fname.clone();
            },
            _ => {}
        }
        match self.json.zones {
            Some(zonecount) => {
                server.worldzone_count = zonecount;
                server.zones = Vec::with_capacity(zonecount * zonecount);
                for _ in 0..(zonecount * zonecount) {
                    server.zones.push(Vec::new());
                }
            }
            _ => {}
        }
    }
}