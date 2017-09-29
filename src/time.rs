
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
    pub fn tick(&mut self) -> (Time, Time) {
        let now = Instant::now();

        let age = (now - self.start).into();
        let delta = (now - self.last).into();

        self.last = now;

        (age, delta)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time(pub u64); 

impl Time {
    pub const ZERO: Time = Time(0);

    pub fn from_ms(ms: u64) -> Time { Time(ms * 1_000_000) } 
    pub fn from_secs(s: u64) -> Time { Time(s * 1_000_000_000) } 
    pub fn from_secs_float(s: f32) -> Time { Time((s * 1_000_000_000.0) as u64) } 

    /// Converts this timing to seconds, truncating any overflow. 1.999 ms will be converted to 1 ms.
    pub fn as_ms(self) -> u64 { self.0 / 1_000_000 }

    /// Converts this timing to seconds, truncating any overflow. 1.999 seconds will be converted to 1.
    pub fn as_secs(self) -> u64 { self.0 / 1_000_000_000 }

    pub fn as_secs_float(self) -> f32 { self.0 as f32 / 1_000_000_000.0 }

    pub fn max(self, other: Time) -> Time {
        ::std::cmp::max(self, other)
    }

    pub fn min(self, other: Time) -> Time {
        ::std::cmp::min(self, other)
    }
}

impl Add for Time {
    type Output = Time;
    fn add(self, rhs: Time) -> Time {
        Time(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Time;
    fn sub(self, rhs: Time) -> Time {
        Time(self.0 - rhs.0)
    }
}

impl AddAssign for Time {
    fn add_assign(&mut self, rhs: Time) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Time {
    fn sub_assign(&mut self, rhs: Time) {
        self.0 -= rhs.0;
    }
}

impl From<Duration> for Time {
    fn from(d: Duration) -> Time {
        Time(d.as_secs()*1_000_000_000 + (d.subsec_nanos() as u64))
    }
}

impl From<Time> for Duration {
    fn from(t: Time) -> Duration {
        let nanos = t.0 % 1_000_000_000;
        let secs = t.0 / 1_000_000_000;
        Duration::new(secs, nanos as u32)
    }
}
