use crate::fixed_index_vec::Pos::{Empty, Reserved};

use self::Pos::*;

/// Defines a position of a [super::FixedIndexVec], where they can be [Used] if they hold a value,
/// [Reserved] if they are reserved for a new value but without having pushed the value yet, or
/// [Empty] if they just don't hold anything
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pos<Value> {
    /// A cell that is empty, being able to fill it by [super::FixedIndexVec::push]
    Empty,
    /// A cell whose value is empty, but its index was reserved, meaning it can only be filled with
    /// [super::FixedIndexVec::push_reserved] indicating its index
    Reserved,
    /// A cell that contains a value of the desired type
    Used(Value),
}

impl<Value> Default for Pos<Value> {
    /// Default value is just [Pos::Empty]
    fn default() -> Self {
        Empty
    }
}

impl<Value> Pos<Value> {
    /// Return if this position is the variant of [Pos::Empty]
    pub fn is_empty(&self) -> bool {
        match self {
            Empty => { true }
            _ => { false }
        }
    }

    /// Return if this position is the variant of [Pos::Used]
    pub fn is_used(&self) -> bool {
        match self {
            Used(_) => { true }
            _ => { false }
        }
    }

    /// Return if this position is the variant of [Pos::Reserved]
    pub fn is_reserved(&self) -> bool {
        match self {
            Reserved => { true }
            _ => { false }
        }
    }

    /// Returns value as option, being [Option::Some] only if this variant was [Pos::Used]
    pub fn opt(self) -> Option<Value> {
        match self {
            Used(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a reference to the contained value as option, being [Option::Some] only if this
    /// variant was [Pos::Used]
    pub fn as_opt_ref(&self) -> Option<&Value> {
        match self {
            Used(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the contained value as option, being [Option::Some] only if
    /// this variant was [Pos::Used]
    pub fn as_opt_mut(&mut self) -> Option<&mut Value> {
        match self {
            Used(value) => Some(value),
            _ => None,
        }
    }
}
