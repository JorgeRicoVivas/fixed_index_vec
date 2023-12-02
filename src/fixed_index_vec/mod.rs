use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::mem;

use compress_result::CompressResult;

use self::pos::Pos;
use self::pos::Pos::*;

/// Defines the result of a [FixedIndexVec::compress]
pub mod compress_result;

/// Defines positions that are stored in [FixedIndexVec]
pub mod pos;

/// Contains specific trait implementations of [FixedIndexVec] that are commonly used by Rust's
/// collections
mod trait_impls;


/// Vec-like structure where indexes are kept for values even when removing others, usually used to
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
    pub fn clean_right(&mut self) {
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
    /// operation is O(n), where n is the number of empty spaces instead of the length of the
    /// complete Vec.
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
    /// spaces, this counts empty and reserved spaces as well as used spaces.
    /// <br>
    /// <br>
    /// If you want to check the amount of a specific kind of position, refer to
    /// [FixedIndexVec::used_spaces_len], [FixedIndexVec::reserved_spaces_len] and
    /// [FixedIndexVec::empty_spaces_len].
    /// <br>
    /// <br>
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