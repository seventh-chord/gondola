
use cable_math::Vec2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Region {
    pub min: Vec2<f32>,
    pub max: Vec2<f32>,
}

impl Region {
    pub fn center(&self) -> Vec2<f32> { (self.min + self.max) / 2.0 } 

    pub fn width(&self) -> f32        { self.max.x - self.min.x }
    pub fn height(&self) -> f32       { self.max.y - self.min.y }

    pub fn size(&self) -> Vec2<f32>   { self.max - self.min }

    /// Checks if the given point is inside this region.
    pub fn contains(&self, p: Vec2<f32>) -> bool {
        p.x > self.min.x && p.x < self.max.x &&
        p.y > self.min.y && p.y < self.max.y
    }

    /// Width divided by height.
    pub fn aspect(&self) -> f32 {
        let size = self.size();
        size.x / size.y
    }

    /// Swaps `min` and `max` along the y axis
    pub fn flip_y(self) -> Region {
        Region {
            min: Vec2::new(self.min.x, self.max.y),
            max: Vec2::new(self.max.x, self.min.y),
        }
    }

    /// Swaps `min` and `max` along the x axis
    pub fn flip_x(self) -> Region {
        Region {
            min: Vec2::new(self.max.x, self.min.y),
            max: Vec2::new(self.min.x, self.max.y),
        }
    }

    /// Returns the region in which this region overlaps the given other region. This might produce
    /// a negative region.
    pub fn overlap(self, other: Region) -> Region {
        Region {
            min: Vec2 {
                x: f32::max(self.min.x, other.min.x),
                y: f32::max(self.min.y, other.min.y),
            },
            max: Vec2 {
                x: f32::min(self.max.x, other.max.x),
                y: f32::min(self.max.y, other.max.y),
            },
        }
    }

    /// Moves `min` to `(0, 0)` but preserves width and height. 
    pub fn unpositioned(self) -> Region {
        Region {
            min: Vec2::zero(),
            max: self.max - self.min,
        }
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn top_right(self) -> Vec2<f32> {
        Vec2::new(self.max.x, self.min.y)
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn bottom_left(self) -> Vec2<f32> {
        Vec2::new(self.min.x, self.max.y)
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn top_left(self) -> Vec2<f32> {
        self.min
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn bottom_right(self) -> Vec2<f32> {
        self.max
    }
}