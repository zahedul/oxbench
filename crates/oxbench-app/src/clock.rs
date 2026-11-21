use oxbench_core::epoch::{DateTime, Utc};

/// Where "now" comes from. Tests pin it, so their output does not change from run to run.
#[derive(Clone, Copy, Debug)]
pub enum Clock {
    System,
    #[cfg_attr(not(test), allow(dead_code, reason = "only tests pin the time"))]
    Fixed(DateTime<Utc>),
}

impl Clock {
    pub fn now(self) -> DateTime<Utc> {
        match self {
            Clock::System => Utc::now(),
            Clock::Fixed(instant) => instant,
        }
    }

    pub fn is_ticking(self) -> bool {
        matches!(self, Clock::System)
    }
}
