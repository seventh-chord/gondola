
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

    /// Finds `time_since_start` but does not affect `time_since_last_tick` which is computed by
    /// `Timer::tick`.
    pub fn time(&self) -> Time {
        (Instant::now() - self.start).into()
    }
}

/// Time, stored as nanoseconds
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time(pub u64); 

impl Time {
    pub const ZERO: Time = Time(0);
    pub const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;
    pub const NANOSECONDS_PER_MILISECOND: u64 = 1_000_000;

    pub fn from_ms(ms: u64) -> Time {
        Time(ms * Time::NANOSECONDS_PER_MILISECOND) 
    }

    pub fn from_secs(s: u64) -> Time {
        Time(s * Time::NANOSECONDS_PER_SECOND) 
    }

    pub fn from_secs_f32(s: f32) -> Time {
        Time((s * Time::NANOSECONDS_PER_SECOND as f32) as u64) 
    }

    /// Converts this timing to milliseconds, truncating any overflow. 1.999 ms will be converted to 1 ms.
    pub fn to_ms(self) -> u64 {
        self.0 / Time::NANOSECONDS_PER_MILISECOND 
    }
    
    pub fn to_ms_f32(self) -> f32 {
        self.0 as f32 / Time::NANOSECONDS_PER_MILISECOND as f32 
    }

    /// Converts this timing to seconds, truncating any overflow. 1.999 seconds will be converted to 1.
    pub fn to_secs(self) -> u64 {
        self.0 / Time::NANOSECONDS_PER_SECOND 
    }

    pub fn to_secs_f32(self) -> f32 {
        self.0 as f32 / Time::NANOSECONDS_PER_SECOND as f32
    }

    pub fn max(self, other: Time) -> Time {
        ::std::cmp::max(self, other)
    }

    pub fn min(self, other: Time) -> Time {
        ::std::cmp::min(self, other)
    }

    /// Interpolates between the two given times. `t = 0` is `self`, and `t = 1` is `other`. Note
    /// that unlike other lerp functions I commonly use, this one clamps `t` to avoid overflows
    /// with unsigned integers.
    pub fn lerp(self, other: Time, t: f32) -> Time {
        if t <= 0.0 {
            return self;
        } else if t >= 1.0 {
            return other;
        }

        if self.0 > other.0 {
            let d = self.0 - other.0;
            Time(other.0 + (d as f32 * (1.0 - t)) as u64)
        } else {
            let d = other.0 - self.0;
            Time(self.0 + (d as f32 * t) as u64)
        }
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
        Time(d.as_secs()*Time::NANOSECONDS_PER_SECOND + (d.subsec_nanos() as u64))
    }
}

impl From<Time> for Duration {
    fn from(t: Time) -> Duration {
        let nanos = t.0 % Time::NANOSECONDS_PER_SECOND;
        let secs = t.0 / Time::NANOSECONDS_PER_SECOND;
        Duration::new(secs, nanos as u32)
    }
}
