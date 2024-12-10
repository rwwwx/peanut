use std::time::Duration;

pub trait DurationExt {
    fn from_minutes(minutes: u64) -> Self;

    fn as_minutes(self) -> u64;
}

impl DurationExt for Duration {
    fn from_minutes(minutes: u64) -> Self {
        Duration::from_secs(minutes * 60)
    }

    fn as_minutes(self) -> u64 {
        (self.as_secs() + 30) / 60
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_minutes() {
        const _5_MINUTES_IN_SECS: u64 = 5 * 60;
        const _5_MINUTES_AND_15_SECS_IN_SECS: u64 = _5_MINUTES_IN_SECS + 15;
        const _5_MINUTES_AND_30_SECS_IN_SECS: u64 = _5_MINUTES_IN_SECS + 30;
        const _5_MINUTES_AND_45_SECS_IN_SECS: u64 = _5_MINUTES_IN_SECS + 45;

        assert_eq!(Duration::from_secs(_5_MINUTES_IN_SECS).as_minutes(), 5);
        assert_eq!(Duration::from_secs(_5_MINUTES_AND_15_SECS_IN_SECS).as_minutes(), 5);
        assert_eq!(Duration::from_secs(_5_MINUTES_AND_30_SECS_IN_SECS).as_minutes(), 6);
        assert_eq!(Duration::from_secs(_5_MINUTES_AND_45_SECS_IN_SECS).as_minutes(), 6);
    }
}