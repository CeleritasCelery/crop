use std::fmt::{self, Debug};

use super::Summarize;

pub(super) struct Leaf<Leaf: Summarize> {
    value: Leaf,
    summary: Leaf::Summary,
}

impl<Leaf: Summarize> Debug for self::Leaf<Leaf> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !f.alternate() {
            f.debug_struct("Leaf")
                .field("value", &self.value)
                .field("summary", &self.summary)
                .finish()
        } else {
            write!(f, "{:?} — {:?}", self.value, self.summary)
        }
    }
}

impl<Leaf: Summarize> self::Leaf<Leaf> {
    pub(super) fn from_value(value: Leaf) -> Self {
        Self { summary: value.summarize(), value }
    }

    pub(super) fn summary(&self) -> &Leaf::Summary {
        &self.summary
    }
}