extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::IntoIter;
use core::mem;
use core::slice::Iter;
use core::iter::Map;
use core::ops::Index;
use core::panic::{RefUnwindSafe, UnwindSafe};
use core::slice::{IterMut, SliceIndex};

//! A Vec structure where indexes are kept for values even when removing others, usually used to
//! replace HashMap<usize, T> on environments where std can't reach or when performance of accessing
//! by index is extremely important but can do get it at the expense of memory allocation,
//! especially when the removal operation is not required or used often as every operation is O(1),
//! where the access is done through a Vec, not requiring hashing operations.
//! <br>
//! <br>
//! This is implemented using a Vec where the values are stored, once a value is inserted, an index
//! to the position they are using is returned, this way, values can be accessed on an O(1) time.
//! <br>
//! <br>
//! # Removing values
//! When values are removed, the position they stored are left as empty, meaning when a new value is
//! inserted, it will take one of the empty positions instead of reallocating, for example, if we
//! assign 5 values, and we remove that of index 3, the structure looks like this:
//! <br>
//! <br>
//! [ValueA, ValueB, ValueC, _, ValueE]
//! <br>
//! <br>
//! <br>
//! If when removing a value, all of it's leading position are also empty, then they get removed,
//! freeing up their memory space, this means if we removed index 3 and 4, the structure wouldn't
//! look like this:
//! <br>
//! <br>
//! [ValueA, ValueB, ValueC, _, _]
//! <br>
//! <br>
//! But like this:
//! <br>
//! <br>
//! [ValueA, ValueB, ValueC]
//! <br>
//! <br>
//! # Clearing up space
//! <br>
//! There might get to a point where there are a lot of Empty spaces and you would like to clear up
//! some memory, but clearing up these values directly would mean most of the indexes would do too,
//! to avoid so, it will swap empty spaces with the last places that have values, meaning just those
//! with high indexes will get their indexes, to trigger this, the operation [VecAssign::compress]
//! will return all the indexes that changed followed by their new indexes, this means if the
//! structure looked like this:
//! <br>
//! <br>
//!
//! [ValueA, _, ValueC, ValueD, ValueE, _, ValueG, ValueH]
//! <br>
//! <br>
//! The operation [FixedIndexVec::compress] would return
//! [(Index 7 changed to 1), (Index 6 changed to 5)] and the structure would look like this:
//! <br>
//! <br>
//! [ValueA, ValueH, ValueC, ValueD, ValueE, ValueG]
//! <br>
//! <br>
//! <br>
//! If you held the indexes in any  structure that can turn into an Iterator<&mut usize>, then the
//! result of the operation offers a function [CompressResult::update_old_indexes] that will change
//! those values in the Iterator that match to a modified index and replace it with the new one.

/// A Vec structure where indexes are kept for values even when removing others, usually used to
/// replace HashMap<usize, T> on environments where std can't reach or when performance of accessing
/// by index is extremely important but can do get it at the expense of memory allocation,
/// especially when the removal operation is not required or used often as every operation is O(1),
/// where the access is done through a Vec, not requiring hashing operations.
#[derive(Clone, Debug, Eq)]
pub struct FixedIndexVec<Value> {
    /// Holds positions where the values are stored, although these positions can also be empty or
    /// reserved.
    values: Vec<Pos<Value>>,
    /// Holds indexes pointing where empty positions are found.
    vacancies: VecDeque<usize>,
    /// Current amount of empty spaces.
    reserved_spaces: usize,
}

#[test]
fn test_clean_right() {
    let v: FixedIndexVec<usize> = FixedIndexVec::from(vec![0_usize, 1, 2, 34, 5]);
    let mut vec_assing = FixedIndexVec::from([0_usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    vec_assing.remove(5);
    vec_assing.remove(8);
    vec_assing.remove(4);
    vec_assing.remove(6);
    vec_assing.remove(3);
    vec_assing.remove(7);
    vec_assing.remove(0);
    vec_assing.remove(2);
    vec_assing.remove(1);

    vec_assing.remove(10);
    println!("{:?}", vec_assing);
    vec_assing.extend(vec![0, 1, 2, 5, 9]);
    println!("{:?}", vec_assing);
}

impl<Value> FixedIndexVec<Value> {
    /// Creates an empty FixedIndexVec.
    pub const fn new() -> FixedIndexVec<Value> {
        Self {
            values: Vec::new(),
            vacancies: VecDeque::new(),
            reserved_spaces: 0,
        }
    }

    /// Pushes the value into the Vec and returns the index where said value was stored, allocating
    /// only if there was no empty space left out by a previous remove operation.
    /// <br>
    /// <br>
    /// This operation is O(1).
    pub fn push(&mut self, value: Value) -> usize {
        match self.vacancies.pop_front() {
            Some(vacant_index) => {
                self.values[vacant_index] = Used(value);
                vacant_index
            }
            None => {
                let index = self.values.len();
                self.values.push(Used(value));
                index
            }
        }
    }

    /// Removes a value from the vec, leaving it's space as empty and ready for other values, being
    /// an O(log n) operation, where n is the number of current empty spaces.
    /// <br>
    /// <br>
    /// If after removing the value the vec has empty positions on it's right(end) bound, it
    /// performs [FixedIndexVec::clean_right], adding it into another O(n) operation, where n is the
    /// amount of leading empty positions on the right end.
    pub fn remove(&mut self, index: usize) -> Option<Value> {
        if index >= self.values.len() || self.values[index].is_empty() { return None; }
        let pos = self.vacancies.partition_point(|&previous_vacancy_index| previous_vacancy_index < index);
        self.vacancies.insert(pos, index);
        let res = mem::take(&mut self.values[index]).opt();
        self.clean_right();
        res
    }

    /// Reserves an index where a value is intended to be stored was stored, allocating only if
    /// there was no empty space left out by a  previous remove operation.
    /// <br>
    /// <br>
    /// Note: Calling [FixedIndexVec::reserve_pos] and [FixedIndexVec::remove_reserved_pos] multiple
    /// times in a row will cause reallocating at most just once, this is because it doesn't clear
    /// up unnecessary empty positions as [FixedIndexVec::remove] would.
    /// <br>
    /// <br>
    /// This operation is O(1).
    pub fn reserve_pos(&mut self) -> usize {
        self.reserved_spaces += 1;
        match self.vacancies.pop_front() {
            Some(vacant_index) => {
                self.values[vacant_index] = Reserved;
                vacant_index
            }
            None => {
                let index = self.values.len();
                self.values.push(Reserved);
                index
            }
        }
    }

    /// Pushes the value over a reserved position that was got through [FixedIndexVec::reserve_pos],
    /// returning the value if the index sent isn't an actual reserved position.
    /// <br>
    /// <br>
    /// This operation is O(1).
    pub fn push_reserved(&mut self, reserved_pos: usize, value: Value) -> Option<Value> {
        if reserved_pos >= self.values.len() || !self.values[reserved_pos].is_reserved() { return Some(value); }
        self.values[reserved_pos] = Used(value);
        self.reserved_spaces -= 1;
        None
    }

    /// Reserves an index where a value is intended to be stored was stored, allocating only if
    /// there was no empty space left out by a  previous remove operation.
    /// <br>
    /// <br>
    /// Note if you call [FixedIndexVec::reserve_pos] and [FixedIndexVec::remove_reserved_pos]
    /// multiple times in a row, it will reallocate at most just once.
    /// <br>
    /// <br>
    /// This operation is O(1).
    pub fn remove_reserved_pos(&mut self, reserved_pos: usize) -> bool {
        if reserved_pos >= self.values.len() || !self.values[reserved_pos].is_reserved() { return false; }
        self.values[reserved_pos] = Empty;
        self.reserved_spaces -= 1;
        true
    }

    /// Clears all the empty values found from the right end bound up to the first value it finds
    /// that is not an empty position, having a worst-case scenario of O(n) if all the values are
    /// empty.
    fn clean_right(&mut self) {
        let leading_empty_poses = self.values.iter().rev().take_while(|pos| pos.is_empty()).count();
        if leading_empty_poses == 0 { return; }
        let first_index_to_remove = self.values.len() - leading_empty_poses;
        for _ in 0..leading_empty_poses {
            self.values.swap_remove(first_index_to_remove);
        }
        self.vacancies.retain(|vacant_index| vacant_index < &first_index_to_remove);
    }

    /// Clears all and every empty space on the Vec while trying to move the least amount of values
    /// as possible.
    /// <br>
    /// <br>
    /// This operation has a worst-case scenario of O(n) where the Vec's first half is full of empty
    /// spaces and the second is full of used spaces, although for most of standard use-cases, the
    /// operation is O(n), where n is the number of empty spaces instead of the length of the whole
    /// Vec.
    pub fn compress(&mut self, save_results: bool) -> CompressResult {
        self.clean_right();
        if self.vacancies.is_empty() { return CompressResult(Vec::new()); }
        let mut index_results = Vec::new();
        let mut end_cursor = self.values.len();
        mem::take(&mut self.vacancies).into_iter().for_each(|vacant| {
            if end_cursor <= vacant { return; }
            end_cursor -= 1;
            if end_cursor <= vacant { return; }
            while self.values[end_cursor].is_empty() {
                end_cursor -= 1;
                if end_cursor <= vacant { return; }
            }
            self.values.swap(vacant, end_cursor);
            if save_results {
                index_results.push((end_cursor, vacant));
            }
        });
        //Since all empty values where now left on the right end, then we can take them out in a go
        self.clean_right();
        CompressResult(index_results)
    }

    /// Clears all positions, whether they are used, reserved or empty, leaving it completely empty.
    pub fn clear(&mut self) {
        self.values.clear();
        self.vacancies.clear();
        self.reserved_spaces = 0;
    }

    /// Returns the amount of spaces used, note this is not the same as the amount of **Used**
    /// spaces, this counts empty and reserved spaces as well as used spaces, if you want to check
    /// the amount of a specific kind of position, refer to [FixedIndexVec::used_spaces_len],
    /// [FixedIndexVec::reserved_spaces_len] and [FixedIndexVec::empty_spaces_len].
    ///
    /// If you want to use iterate over used spaces, use [FixedIndexVec::iter],
    /// [FixedIndexVec::iter_mut] and [FixedIndexVec::into_iter].
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Amount of positions holding a value.
    pub fn used_spaces_len(&self) -> usize {
        self.values.len() - (self.reserved_spaces_len() + self.empty_spaces_len())
    }

    /// Amount of reserved positions waiting for a value to get pushed.
    pub fn reserved_spaces_len(&self) -> usize {
        self.reserved_spaces
    }

    /// Amount of empty positions.
    pub fn empty_spaces_len(&self) -> usize {
        self.vacancies.len()
    }

    /// Returns whether this index holds a value or not.
    pub fn contains_index(&self, index: usize) -> bool {
        index < self.values.len() && self.values[index].is_used()
    }

    /// Returns a reference to the value matching this index.
    pub fn get(&self, index: usize) -> Option<&Value> {
        if !self.contains_index(index) { return None; }
        self.values[index].as_opt_ref()
    }

    /// Returns a mutable reference to the value matching this index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        if !self.contains_index(index) { return None; }
        self.values[index].as_opt_mut()
    }

    /// Iterator over all stored value (This excludes empty and reserved positions).
    pub fn iter(&self) -> impl Iterator<Item=&Value> {
        self.values.iter()
            .filter(|pos| pos.is_used())
            .map(|pos| match pos {
                Used(value) => value,
                _ => panic!()
            })
    }

    /// Mutable iterator over all stored value (This excludes empty and reserved positions).
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut Value> {
        self.values.iter_mut()
            .filter(|pos| pos.is_used())
            .map(|pos| match pos {
                Used(value) => value,
                _ => panic!()
            })
    }

    /// In-Place iterator over all stored value (This excludes empty and reserved positions).
    pub fn into_iter(self) -> impl Iterator<Item=Value> {
        self.values.into_iter()
            .filter(|pos| pos.is_used())
            .map(|pos| match pos {
                Used(value) => value,
                _ => panic!()
            })
    }
}

/// Holds the result of an [FixedIndexVec]'s compression, each value matches the old index to the
/// new one.
pub struct CompressResult(pub Vec<(usize, usize)>);

impl CompressResult {
    /// Replaces all the old indexes found in this iterator with the new indexes after a
    /// compression.
    pub fn update_old_indexes<'old_indexes, IndexType>(&self, old_indexes_iter: impl Iterator<Item=&'old_indexes mut IndexType>)
        where IndexType: From<usize> + PartialEq<usize> + 'old_indexes {
        old_indexes_iter.for_each(|value| {
            match self.0.iter().filter(|(changed_index, _)| *value == *changed_index).next() {
                None => {}
                Some((_, new_index)) => { *value = (*new_index).into() }
            }
        })
    }
}

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
        !self.reserved_spaces(other)
    }
}


impl<Value: RefUnwindSafe> RefUnwindSafe for FixedIndexVec<Value> {}

unsafe impl<Value: Send> Send for FixedIndexVec<Value> {}

unsafe impl<Value: Sync> Sync for FixedIndexVec<Value> {}

impl<Value: Unpin> Unpin for FixedIndexVec<Value> {}

impl<Value: UnwindSafe> UnwindSafe for FixedIndexVec<Value> {}

use self::Pos::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pos<T> {
    Empty,
    Reserved,
    Used(T),
}

impl<T> Default for Pos<T> {
    fn default() -> Self {
        Empty
    }
}

impl<T> Pos<T> {
    fn is_empty(&self) -> bool {
        match self {
            Empty => { true }
            _ => { false }
        }
    }
    fn is_used(&self) -> bool {
        match self {
            Used(_) => { true }
            _ => { false }
        }
    }
    fn is_reserved(&self) -> bool {
        match self {
            Reserved => { true }
            _ => { false }
        }
    }
    fn opt(self) -> Option<T> {
        match self {
            Used(value) => Some(value),
            _ => None,
        }
    }
    fn as_opt_ref(&self) -> Option<&T> {
        match self {
            Used(value) => Some(value),
            _ => None,
        }
    }
    fn as_opt_mut(&mut self) -> Option<&mut T> {
        match self {
            Used(value) => Some(value),
            _ => None,
        }
    }
}