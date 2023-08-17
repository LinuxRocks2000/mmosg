// This is mulberry32 in rust. It's not a very high quality rng (even the creator noticed it's full of problems) but it suits my needs. The important bit is that it's Send + Sync so I can arcmutex it.
use std::num::Wrapping;


#[derive(Copy, Clone)]
pub struct Mulberry32 {
    state : Wrapping<u32>
}

impl Mulberry32 {
    pub fn new(seed : u32) -> Self {
        Self {
            state : Wrapping(seed)
        }
    }

    pub fn next(&mut self) -> u32 {
        self.state += 0x6D2B79F5;
        let mut z = self.state;
        z = (z ^ (z >> 15)) * Wrapping(z.0 | 1);
        z ^= z + (z ^ (z >> 7)) * Wrapping(z.0 | 61);
        z.0 ^ (z.0 >> 14)
    }
}