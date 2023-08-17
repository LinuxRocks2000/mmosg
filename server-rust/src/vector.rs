// Rust-ic vector library, equivalent to vector.hpp
use std::f32::consts::PI;


#[derive(Copy, Clone)]
pub struct Vector2 {
    pub x : f32,
    pub y : f32,
}

impl Vector2 {
    pub fn empty() -> Self {
        Self {
            x : 0.0,
            y : 0.0
        }
    }

    pub fn new(x : f32, y : f32) -> Self {
        Self {
            x,
            y
        }
    }

    pub fn new_from_manda(mag : f32, ang : f32) -> Self {
        Vector2::new (mag * ang.cos(), mag * ang.sin())
    }

    pub fn angle(&self) -> f32 {
        self.y.atan2(self.x)
    }

    pub fn is_zero(&self) -> bool {
        self.y == 0.0 && self.x == 0.0
    }

    pub fn set_manda(&mut self, mag : f32, ang : f32) {
        self.x = ang.cos() * mag;
        self.y = ang.sin() * mag;
    }

    pub fn set_angle(&mut self, ang : f32){
        self.set_manda(self.magnitude(), ang);
    }

    pub fn set_angle_degs(&mut self, ang : f32) {
        self.set_angle(ang * std::f32::consts::PI/180.0);
    }

    pub fn set_magnitude(&mut self, mag : f32){
        self.set_manda(mag, self.angle());
    }

    pub fn ang_rads(&self) -> f32 {
        self.angle()
    }

    pub fn ang_degs(&self) -> f32 {
        self.angle() * 180.0 / std::f32::consts::PI
    }

    pub fn mag2(&self) -> f32 {
        self.y * self.y + self.x * self.x
    }

    pub fn magnitude(&self) -> f32 {
        self.mag2().sqrt()
    }

    pub fn lim(&mut self, max_mag : f32){
        let magnitude = self.magnitude();
        if magnitude > max_mag {
            self.set_magnitude(max_mag);
        }
    }

    pub fn rot(&self, ang : f32) -> Vector2 {
        let mut v = Vector2::new (self.x, self.y);
        v.set_angle(v.angle() + ang);
        v
    }

    pub fn dot(&self, onto : Vector2) -> f32 {
        let angle_between = self.angle() - onto.angle();
        angle_between.cos() * self.magnitude()
    }

    pub fn project(&self, onto : Vector2) -> Vector2 {
        Vector2::new_from_manda(self.dot(onto), onto.angle())
    }

    pub fn modsnap(&self, to : f32) -> Vector2 {
        let mut r = Vector2::new(self.x, self.y);
        let x_snap = r.x % to;
        let y_snap = r.y % to;
        r.x = r.x - x_snap;
        r.y = r.y - y_snap;
        if x_snap > to / 2.0 {
            r.x += 1.0;
        }
        if y_snap > to / 2.0 {
            r.y += 1.0;
        }
        r
    }

    pub fn unit(&self) -> Vector2 {
        Self::new_from_manda(1.0, self.angle())
    }

    pub fn cut(&self, about : Vector2) -> (Vector2, Vector2) {
        (self.project(about), self.project(about.rot(PI/2.0)))
    }

    pub fn is_basically(&self, mag : f32) -> bool {
        (self.magnitude() - mag).abs() < 0.001
    }

    pub fn sqrt(&self) -> Vector2 {
        Vector2::new_from_manda(self.magnitude().sqrt(), self.angle())
    }
}

impl std::fmt::Debug for Vector2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vector2")
        .field("x", &self.x)
        .field("y", &self.y)
        .field("magnitude", &self.magnitude())
        .field("angle", &self.angle())
        .finish()
    }
}

impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl std::ops::Add <Vector2> for Vector2 {
    type Output = Vector2;

    fn add (self, other : Vector2) -> Vector2 {
        Vector2 {
            x : self.x + other.x,
            y : self.y + other.y
        }
    }
}

impl std::ops::AddAssign <Vector2> for Vector2 {
    fn add_assign (&mut self, other : Vector2) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl std::ops::Sub <Vector2> for Vector2 {
    type Output = Vector2;

    fn sub (self, other : Vector2) -> Vector2 {
        Vector2 {
            x : self.x - other.x,
            y : self.y - other.y
        }
    }
}

impl std::ops::Mul <f32> for Vector2 {
    type Output = Vector2;

    fn mul (self, value : f32) -> Vector2 {
        Vector2 {
            x : self.x * value,
            y : self.y * value
        }
    }
}

impl std::ops::Div <f32> for Vector2 {
    type Output = Vector2;

    fn div (self, value : f32) -> Vector2 {
        Vector2 {
            x : self.x / value,
            y : self.y / value
        }
    }
}

impl std::ops::MulAssign <f32> for Vector2 {
    fn mul_assign(&mut self, value : f32) {
        self.x *= value;
        self.y *= value;
    }
}