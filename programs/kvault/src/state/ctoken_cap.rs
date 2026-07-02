

#[derive(Clone, Copy, Debug, Default)]
pub struct CtokenCap(u64);

impl CtokenCap {
    pub const UNCAPPED: Self = Self(0);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn uncapped() -> Self {
        Self::UNCAPPED
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn is_uncapped(self) -> bool {
        self.0 == 0 || self.0 == u64::MAX
    }
}

impl PartialEq for CtokenCap {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for CtokenCap {}

impl PartialOrd for CtokenCap {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CtokenCap {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self.is_uncapped(), other.is_uncapped()) {
            (true, true) => core::cmp::Ordering::Equal,
            (true, false) => core::cmp::Ordering::Greater,
            (false, true) => core::cmp::Ordering::Less,
            (false, false) => self.0.cmp(&other.0),
        }
    }
}
