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
struct ServerConfigFile {
    password        : Option<String>,
    world_size      : u32,
    io_mode         : Option<bool>,
    prompt_password : Option<bool>,
    map             : Vec<ObjectDef>,
    autonomous      : Option<AutonomousDef>,
    teams           : Option<Vec<TeamDef>>,
    strat_secs      : Option<f32>,
    play_secs       : Option<f32>,
    headless        : Option<bool>,
    permit_npcs     : Option<bool>,
    port            : Option<u16>
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

    pub async fn load_into(&self, server : &mut Server) {
        server.gamesize = self.json.world_size;
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
                server.new_team(team.name.clone(), team.password.clone()).await;
            }
        }
        for def in &self.json.map {
            server.place_block(def.x, def.y, match def.a {
                Some(a) => a,
                None => 0.0
            }, def.w, def.h).await;
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
            }
            _ => {}
        }
        match self.json.play_secs {
            Some(time) => {
                server.times.1 = time;
            }
            _ => {}
        }
    }
}