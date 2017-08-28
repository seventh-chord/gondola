
use std::time::{Instant, Duration};
use std::ops::{Add, Sub, AddAssign, SubAssign};

/// Utility to track time in a program
#[derive(Clone)]
pub struct Timer {
    start: Instant,
    last: Instant,
}

impl Timer {
    pub fn new() -> Timer {
        let now = Instant::now();

        Timer {
            start: now,
            last: now,
        }
    }

    /// Returns `(time_since_start, time_since_last_tick)`
    pub fn tick(&mut self) -> (Timing, Timing) {
        let now = Instant::now();

        let age = (now - self.start).into();
        let delta = (now - self.last).into();

        self.last = now;

        (age, delta)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timing(pub u64); 

impl Timing {
    pub fn zero() -> Timing { Timing(0) }

    pub fn from_ms(ms: u64) -> Timing { Timing(ms * 1_000_000) } 
    pub fn from_secs(s: u64) -> Timing { Timing(s * 1_000_000_000) } 
    pub fn from_secs_float(s: f32) -> Timing { Timing((s * 1_000_000_000.0) as u64) } 

    /// Converts this timing to seconds, truncating any overflow. 1.999 ms will be converted to 1 ms.
    pub fn as_ms(self) -> u64 { self.0 / 1_000_000 }

    /// Converts this timing to seconds, truncating any overflow. 1.999 seconds will be converted to 1.
    pub fn as_secs(self) -> u64 { self.0 / 1_000_000_000 }

    pub fn as_secs_float(self) -> f32 { self.0 as f32 / 1_000_000_000.0 }

    pub fn max(self, other: Timing) -> Timing {
        ::std::cmp::max(self, other)
    }

    pub fn min(self, other: Timing) -> Timing {
        ::std::cmp::min(self, other)
    }
}

impl Add for Timing {
    type Output = Timing;
    fn add(self, rhs: Timing) -> Timing {
        Timing(self.0 + rhs.0)
    }
}

impl Sub for Timing {
    type Output = Timing;
    fn sub(self, rhs: Timing) -> Timing {
        Timing(self.0 - rhs.0)
    }
}

impl AddAssign for Timing {
    fn add_assign(&mut self, rhs: Timing) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Timing {
    fn sub_assign(&mut self, rhs: Timing) {
        self.0 -= rhs.0;
    }
}

impl From<Duration> for Timing {
    fn from(d: Duration) -> Timing {
        Timing(d.as_secs()*1_000_000_000 + (d.subsec_nanos() as u64))
    }
}

impl From<Timing> for Duration {
    fn from(t: Timing) -> Duration {
        let nanos = t.0 % 1_000_000_000;
        let secs = t.0 / 1_000_000_000;
        Duration::new(secs, nanos as u32)
    }
}
