use core::fmt::{self, Debug, Display};
use core::cmp::Ordering;

#[derive(Clone, Copy, Debug, Hash)]
pub enum Length<Idx> {
    Unbounded,
    Bounded(Idx),
}

impl<Idx: Copy> Length<Idx> {
    pub fn try_len(&self) -> Option<Idx> {
        match *self {
            Length::Unbounded => None,
            Length::Bounded(n) => Some(n),
        }
    }

    pub unsafe fn len(&self) -> Idx {
        match *self {
            Length::Unbounded => panic!(
                "attempt to convert `Length::Unbounded` to `Length::Idx`"
            ),
            Length::Bounded(n) => n,
        }
    }
}

impl<Idx> Length<Idx> {
    pub fn is_bounded(&self) -> bool {
        match *self {
            Length::Unbounded => false,
            Length::Bounded(_) => true,
        }
    }
}

impl<Idx: Debug> Display for Length<Idx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl<Idx: PartialEq> PartialEq for Length<Idx> {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&Length::Unbounded, _) => false,
            (_, &Length::Unbounded) => false,
            (&Length::Bounded(ref a), &Length::Bounded(ref b)) => a.eq(b),
        }
    }

    fn ne(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&Length::Unbounded, _) => false,
            (_, &Length::Unbounded) => false,
            (&Length::Bounded(ref a), &Length::Bounded(ref b)) => a.ne(b),
        }
    }
}

impl<Idx: PartialEq> PartialEq<Idx> for Length<Idx> {
    fn eq(&self, rhs: &Idx) -> bool {
        match *self {
            Length::Unbounded => false,
            Length::Bounded(ref n) => n.eq(rhs),
        }
    }

    fn ne(&self, rhs: &Idx) -> bool {
        match *self {
            Length::Unbounded => false,
            Length::Bounded(ref n) => n.eq(rhs),
        }
    }
}

impl<Idx: PartialOrd> PartialOrd for Length<Idx> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        match (self, rhs) {
            (&Length::Unbounded, &Length::Unbounded) => None,
            (&Length::Unbounded, _) => Some(Ordering::Greater),
            (_, &Length::Unbounded) => Some(Ordering::Less),
            (&Length::Bounded(ref a), &Length::Bounded(ref b)) => {
                a.partial_cmp(b)
            }
        }
    }
}

impl<Idx: PartialOrd> PartialOrd<Idx> for Length<Idx> {
    fn partial_cmp(&self, rhs: &Idx) -> Option<Ordering> {
        match *self {
            Length::Unbounded => Some(Ordering::Greater),
            Length::Bounded(ref n) => n.partial_cmp(rhs),
        }
    }
}
