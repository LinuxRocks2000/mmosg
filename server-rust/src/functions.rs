// Useful functions on the top level


use std::f32::consts::PI;


pub fn coterminal<T: Copy + std::ops::Add<U, Output=T> + std::ops::Sub<U, Output=T> + std::cmp::PartialOrd<U>, U: Copy + std::cmp::PartialOrd<T> + std::ops::Sub<U, Output=U>>(mut thing : T, round : U) -> T {
    while thing < (round - round) {
        thing = thing + round;
    }
    while thing > round {
        thing = thing - round;
    }
    thing
}

pub fn loopize_about(set : f32, cur : f32, about : f32) -> f32 {
    if (set - cur).abs() >= about / 2.0 {
        if set > cur {
            return -(about - set + cur);
        }
        else {
            return about - cur + set;
        }
    }
    set - cur
}

pub fn loopize(set : f32, cur : f32) -> f32 {
    /*set - cur*/loopize_about(set, cur, PI * 2.0)
}