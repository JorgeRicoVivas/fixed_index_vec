use alloc::vec::IntoIter;
use core::iter::Map;
use core::panic::{RefUnwindSafe, UnwindSafe};
use core::slice::{Iter, IterMut, SliceIndex};

use crate::fixed_index_vec::FixedIndexVec;
use crate::fixed_index_vec::pos::Pos;
use crate::fixed_index_vec::pos::Pos::Used;

impl<Value> Default for FixedIndexVec<Value> {
    /// Creates an empty [FixedIndexVec], this is the same as calling [FixedIndexVec::new].
    fn default() -> Self {
        Self::new()
    }
}

impl<'value, Value: Clone> Extend<&'value Value> for FixedIndexVec<Value> {
    /// Extends the values from the iterator by cloning them applying [FixedIndexVec::push] on every
    /// value.
    fn extend<T: IntoIterator<Item=&'value Value>>(&mut self, iter: T) {
        Extend::<Value>::extend(self, iter.into_iter().map(Clone::clone))
    }
}

impl<Value> Extend<Value> for FixedIndexVec<Value> {
    /// Extends the values from the iterator by applying [FixedIndexVec::push] on every value.
    fn extend<T: IntoIterator<Item=Value>>(&mut self, iter: T) {
        iter.into_iter().for_each(|value| {
            match self.vacancies.pop_front() {
                Some(vacant_index) => self.values[vacant_index] = Used(value),
                None => self.values.push(Used(value)),
            }
        })
    }
}

impl<Value, ValueIterator> From<ValueIterator> for FixedIndexVec<Value>
    where ValueIterator: IntoIterator<Item=Value> {
    /// Creates a new [FixedIndexVec] where every position is initially occupied by the items from
    /// the iterator.
    fn from(values: ValueIterator) -> Self {
        Self {
            values: values.into_iter().map(|value| Used(value)).collect(),
            ..Self::new()
        }
    }
}

impl<Value> FromIterator<Value> for FixedIndexVec<Value> {
    /// Creates a new [FixedIndexVec] where every position is initially occupied by the items from
    /// the iterator.
    ///
    /// This is done through [From::from].
    fn from_iter<T: IntoIterator<Item=Value>>(iter: T) -> Self {
        Self::from(iter)
    }
}

impl<Value, Index> core::ops::Index<Index> for FixedIndexVec<Value> where Index: SliceIndex<[Pos<Value>]>, {
    type Output = Index::Output;

    /// Obtains a reference to the position corresponding to this value.
    ///
    /// Note this is not the same as a value, as [Pos] also represent empty and reserved positions,
    /// not just positions filled with values.
    fn index(&self, index: Index) -> &Self::Output {
        core::ops::Index::index(&self.values, index)
    }
}

impl<Value, Index> core::ops::IndexMut<Index> for FixedIndexVec<Value> where Index: SliceIndex<[Pos<Value>]>, {
    /// Obtains a mutable reference to the position corresponding to this value.
    ///
    /// Note this is not the same as a value, as [Pos] also represent empty and reserved positions,
    /// not just positions filled with values.
    fn index_mut(&mut self, index: Index) -> &mut Self::Output {
        core::ops::IndexMut::index_mut(&mut self.values, index)
    }
}


impl<'selflf, Value> IntoIterator for &'selflf FixedIndexVec<Value> {
    type Item = Option<&'selflf Value>;
    type IntoIter = Map<Iter<'selflf, Pos<Value>>, fn(&Pos<Value>) -> Option<&Value>>;

    /// Gets an iterator over referenced positions from this [FixedIndexVec], note positions might
    /// be empty, returning [Option::None], if you want to transverse through just values and not
    /// empty or reserved positions, use [FixedIndexVec::iter] instead
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter().map(Pos::as_opt_ref)
    }
}

impl<'selflf, Value> IntoIterator for &'selflf mut FixedIndexVec<Value> {
    type Item = Option<&'selflf mut Value>;
    type IntoIter = Map<IterMut<'selflf, Pos<Value>>, fn(&mut Pos<Value>) -> Option<&mut Value>>;

    /// Gets an iterator over mutable references from the positions from this [FixedIndexVec], note
    /// positions might be empty, returning [Option::None], if you want to transverse through just
    /// values and not empty or reserved positions, use [FixedIndexVec::iter_mut] instead
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter_mut().map(Pos::as_opt_mut)
    }
}

impl<'selflf, Value> IntoIterator for FixedIndexVec<Value> {
    type Item = Option<Value>;
    type IntoIter = Map<IntoIter<Pos<Value>>, fn(Pos<Value>) -> Option<Value>>;

    /// Turns this [FixedIndexVec] into an iterator over its positions, note positions might be
    /// empty, returning [Option::None], if you want to transverse through just values and not empty
    /// or reserved positions, use [FixedIndexVec::into_iter] instead
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter().map(Pos::opt)
    }
}

impl<Value: PartialEq> PartialEq for FixedIndexVec<Value> {
    /// Compares if two [FixedIndexVec] contents are equal, trying to compare them in the most
    /// efficient way.
    fn eq(&self, other: &Self) -> bool {
        let lengths_are_equal = self.reserved_spaces.eq(&other.reserved_spaces)
            && self.vacancies.len().eq(&other.vacancies.len())
            && self.values.len().eq(&other.values.len());
        if !lengths_are_equal { return false; };
        if self.used_spaces_len() > self.vacancies.len() {
            self.values.eq(&other.values)
                && self.vacancies.eq(&other.vacancies)
        } else {
            self.vacancies.eq(&other.vacancies)
                && self.values.eq(&other.values)
        }
    }

    /// Compares if two [FixedIndexVec] contents are different, trying to compare them in the most
    /// efficient way.
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}


impl<Value: RefUnwindSafe> RefUnwindSafe for FixedIndexVec<Value> {}

unsafe impl<Value: Send> Send for FixedIndexVec<Value> {}

unsafe impl<Value: Sync> Sync for FixedIndexVec<Value> {}

impl<Value: Unpin> Unpin for FixedIndexVec<Value> {}

impl<Value: UnwindSafe> UnwindSafe for FixedIndexVec<Value> {}
