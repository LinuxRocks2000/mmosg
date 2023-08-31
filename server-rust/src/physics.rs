use crate::vector::Vector2;
use std::f32::consts::PI;

#[derive(Copy, Clone, Debug)]
pub struct BoxShape {
    pub x : f32,
    pub y : f32,
    pub w : f32,
    pub h : f32,
    pub a : f32
}

impl BoxShape {
    pub fn empty() -> Self {
        Self {
            w : 0.0,
            h : 0.0,
            x : 0.0,
            y : 0.0,
            a : 0.0
        }
    }

    pub fn worst(&self) -> BoxShape { // The goal here is not to get an accurate bounding box, just to get a rough bounding box that is certain to contain the actual rectangle and get it really fast
        let long = self.w + self.h; // This is guaranteed to be longer than the longest straight line you can fit in the rectangle.
        BoxShape {
            x : self.x, 
            y : self.y,
            w : long,
            h : long,
            a : 0.0
        }
    }

    pub fn get_perp_axes(&self) -> Vec<Vector2> {
        vec![
            Vector2::new_from_manda(1.0, self.a), // No need to perpendicularize these two - they *are* their own perpendiculars.
            Vector2::new_from_manda(1.0, self.a + PI/2.0) // For the other two sides, the angle comes out the same, and the angle is the only important component. Thus: optimize by leaving 'em out.
        ]
    }

    pub fn points(&self) -> Vec<Vector2> {
        let to_origin = Vector2::new(self.x, self.y);
        let o_tl = Vector2::new(-self.w/2.0, -self.h/2.0);
        let o_tr = Vector2::new(self.w/2.0, -self.h/2.0);
        let o_bl = Vector2::new(-self.w/2.0, self.h/2.0);
        let o_br = Vector2::new(self.w/2.0, self.h/2.0);
        vec![
            o_tl.rot(self.a) + to_origin,
            o_tr.rot(self.a) + to_origin,
            o_bl.rot(self.a) + to_origin,
            o_br.rot(self.a) + to_origin,
        ]
    }

    pub fn get_dotrange(&self, axis : Vector2) -> [f32; 2] {
        let mut ret : [f32; 2] = [0.0, 0.0];
        let points = self.points();
        for (i, point) in points.iter().enumerate() {
            let v = point.dot(axis);
            if i == 0 {
                ret[0] = v;
                ret[1] = v;
            }
            else {
                if v < ret[0] {
                    ret[0] = v;
                }
                if v > ret[1] {
                    ret[1] = v;
                }
            }
        }
        ret
    }

    pub fn intersects(&self, other : BoxShape) -> (bool, Vector2) {
        let mbx = self.worst();
        let tbx = other.worst();
        let mut mtv = Vector2::empty();
        if (mbx.x - mbx.w/2.0 < tbx.x + tbx.w/2.0) && (mbx.y - mbx.h/2.0 < tbx.y + tbx.h/2.0) && (mbx.x + mbx.w/2.0 > tbx.x - tbx.w/2.0) && (mbx.y + mbx.h/2.0 > tbx.y - tbx.h/2.0) { // Short circuit: if there's no fast, crappy collision between the two, as is the case 90% of the time, don't bother doing a slow, accurate collision
            let mut axes : Vec<Vector2> = self.get_perp_axes();
            let mut other_axes : Vec<Vector2> = other.get_perp_axes();
            axes.append(&mut other_axes);
            for (_, axis) in axes.iter().enumerate() {
                let me_range = self.get_dotrange(*axis);
                let them_range = other.get_dotrange(*axis);
                if (me_range[0] >= them_range[1]) || (me_range[1] <= them_range[0]) { // If on any axis it doesn't intersect, there's no collision at all
                    return (false, Vector2::empty()); // Short circuit
                }
                let m_low = me_range[0] - them_range[1];
                let m_high = me_range[1] - them_range[0];
                let m_choice = if m_low.abs() < m_high.abs() {
                    m_low
                } else {
                    m_high
                } * -1.0; // Note: in both cases the value you get is wrong, by a predictable factor of -1. We can reverse that very easily.
                let vectah = Vector2::new_from_manda(m_choice, axis.angle()); // Create a vector about the axis with magnitude m_choice
                if mtv.is_zero() || vectah.magnitude() < mtv.magnitude() {
                    mtv = vectah;
                }
                // Note: because of the above intersection check, this will never issue a translation vector that wouldn't pull out.
            }
            return (true, mtv);
        }
        (false, Vector2::empty())
    }

    pub fn translate(&mut self, velocity : Vector2) {
        self.x += velocity.x;
        self.y += velocity.y;
    }

    pub fn rotate(&mut self, velocity : f32) {
        self.a += velocity;
    }

    pub fn from_corners(x1 : f32, y1 : f32, x2 : f32, y2 : f32) -> Self {
        Self {
            x : (x1 + x2) / 2.0,
            y : (y1 + y2) / 2.0,
            w : x2 - x1,
            h : y2 - y1,
            a : 0.0
        }
    }

    pub fn ong_fr(&self) -> BoxShape { // create a high-quality bounding box of this BoxShape, but slower than worst()
        let points = self.points();
        let mut lowest_x = self.x;
        let mut lowest_y = self.y;
        let mut highest_x = self.x;
        let mut highest_y = self.y;
        for point in points {
            if point.x > highest_x {
                highest_x = point.x;
            }
            if point.x < lowest_x {
                lowest_x = point.x;
            }
            if point.y > highest_y {
                highest_y = point.y;
            }
            if point.y < lowest_y {
                lowest_y = point.y;
            }
        }
        Self::from_corners(lowest_x, lowest_y, highest_x, highest_y)
    }

    pub fn bigger(&self, amount : f32) -> Self {
        Self {
            x : self.x,
            y : self.y, 
            w : self.w + amount,
            h : self.h + amount,
            a : self.a
        }
    }

    pub fn contains(&self, point : Vector2) -> bool {
        let cmp = if self.a != 0.0 {
            point.rotate_about(Vector2::new(self.x, self.y), -self.a)
        }
        else {
            point
        };
        cmp.x > self.x - self.w/2.0 && cmp.x < self.x + self.w/2.0 && cmp.y > self.y - self.h/2.0 && cmp.y < self.y + self.h/2.0
    }
}


#[derive(Clone)]
pub struct PhysicsObject {
    pub shape         : BoxShape,
    pub old_shape     : BoxShape,
    pub velocity      : Vector2,
    pub solid         : bool,
    pub angle_v       : f32,
    pub mass          : f32,
    pub fixed         : bool,
    pub restitution   : f32,
    pub portals       : bool,
    pub speed_cap     : f32
}


impl PhysicsObject {
    pub fn new(x : f32, y : f32, w : f32, h : f32, a : f32) -> Self { // Nonradioactive object
        Self {
            shape : BoxShape {
                x, y, w, h, a
            },
            old_shape : BoxShape::empty(),
            velocity : Vector2::empty(),
            solid : false, // Everything is solid by default
            angle_v : 0.0,
            fixed : false,
            mass : w * h, // Assume a density of 1. If you want to change the *density* elsewhere, just multiply it by the new density!
            restitution : 0.5, // Assume collisions are truly inelastic by default
            portals : false,
            speed_cap : 0.0
        }
    }

    pub fn shape(&self) -> BoxShape {
        self.shape
    }

    pub fn old_shape(&self) -> BoxShape {
        self.old_shape
    }

    pub fn translated(&self) -> bool {
        self.old_shape.x != self.shape.x || self.old_shape.y != self.shape.y
    }

    pub fn rotated(&self) -> bool {
        self.old_shape.a != self.shape.a
    }

    pub fn resized(&self) -> bool {
        self.old_shape.w != self.shape.w || self.old_shape.h != self.shape.h
    }

    pub fn update(&mut self) { // Since this is "newtonian", you should never directly change x and y, and instead change the velocity vector.
        if !self.fixed {
            self.old_shape = self.shape;
            self.shape.translate(self.velocity);
            self.shape.rotate(self.angle_v);
        }
    }

    pub fn cx(&self) -> f32 {
        self.shape.x
    }

    pub fn cy(&self) -> f32 {
        self.shape.y
    }

    pub fn angle(&self) -> f32 {
        self.shape.a
    }

    pub fn thrust(&mut self, amount : f32) {
        self.velocity += Vector2::new_from_manda(amount, self.angle());
    }

    pub fn set_cx(&mut self, x : f32) {
        self.shape.x = x;
    }

    pub fn set_cy(&mut self, y : f32) {
        self.shape.y = y;
    }

    pub fn set_angle(&mut self, a : f32) {
        self.shape.a = a;
    }

    pub fn change_angle(&mut self, by : f32) {
        self.shape.a += by;
    }

    pub fn width(&self) -> f32 {
        self.shape.w
    }

    pub fn height(&self) -> f32 {
        self.shape.h
    }

    pub fn vector_position(&self) -> Vector2 {
        Vector2::new(self.cx(), self.cy())
    }

    pub fn extend_point(&self, amount : f32, off : f32) -> Vector2 {
        self.vector_position() + Vector2::new_from_manda(amount, self.angle() + off)
    }
}