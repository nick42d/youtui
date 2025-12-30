#[derive(Eq, PartialEq, Debug)]
pub struct Constraint<Cstrnt> {
    pub(crate) constraint_type: ConstraitType<Cstrnt>,
}

impl<Cstrnt> Constraint<Cstrnt> {
    pub fn new_block_same_type() -> Self {
        Self {
            constraint_type: ConstraitType::BlockSameType,
        }
    }
    pub fn new_kill_same_type() -> Self {
        Self {
            constraint_type: ConstraitType::KillSameType,
        }
    }
    pub fn new_block_matching_metadata(metadata: Cstrnt) -> Self {
        Self {
            constraint_type: ConstraitType::BlockMatchingMetatdata(metadata),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum ConstraitType<Cstrnt> {
    BlockSameType,
    KillSameType,
    BlockMatchingMetatdata(Cstrnt),
}
