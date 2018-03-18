use core::fmt::{self, Debug, Display};
use core::cmp::Ordering;

#[derive(Clone, Copy, Debug, Hash)]
pub enum Length {
    Unbounded,
    Bounded(usize),
}

impl Length {
    pub fn try_len(&self) -> Option<usize> {
        match *self {
            Length::Unbounded => None,
            Length::Bounded(n) => Some(n),
        }
    }

    pub unsafe fn len(&self) -> usize {
        match *self {
            Length::Unbounded => {
                panic!("attempt to convert `Length::Unbounded` to `usize`")
            }
            Length::Bounded(n) => n,
        }
    }

    pub fn is_bounded(&self) -> bool {
        match *self {
            Length::Unbounded => false,
            Length::Bounded(_) => true,
        }
    }
}

impl Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl PartialEq for Length {
    fn eq(&self, rhs: &Length) -> bool {
        match (*self, *rhs) {
            (Length::Unbounded, _) => false,
            (_, Length::Unbounded) => false,
            (Length::Bounded(a), Length::Bounded(ref b)) => a.eq(b),
        }
    }

    fn ne(&self, rhs: &Length) -> bool {
        match (*self, *rhs) {
            (Length::Unbounded, _) => false,
            (_, Length::Unbounded) => false,
            (Length::Bounded(a), Length::Bounded(ref b)) => a.ne(b),
        }
    }
}

impl PartialEq<usize> for Length {
    fn eq(&self, rhs: &usize) -> bool {
        match *self {
            Length::Unbounded => false,
            Length::Bounded(n) => n.eq(rhs),
        }
    }

    fn ne(&self, rhs: &usize) -> bool {
        match *self {
            Length::Unbounded => false,
            Length::Bounded(n) => n.eq(rhs),
        }
    }
}

impl PartialOrd for Length {
    fn partial_cmp(&self, rhs: &Length) -> Option<Ordering> {
        match (*self, *rhs) {
            (Length::Unbounded, Length::Unbounded) => None,
            (Length::Unbounded, _) => Some(Ordering::Greater),
            (_, Length::Unbounded) => Some(Ordering::Less),
            (Length::Bounded(a), Length::Bounded(ref b)) => a.partial_cmp(b),
        }
    }
}

impl PartialOrd<usize> for Length {
    fn partial_cmp(&self, rhs: &usize) -> Option<Ordering> {
        match *self {
            Length::Unbounded => Some(Ordering::Greater),
            Length::Bounded(n) => n.partial_cmp(rhs),
        }
    }
}
