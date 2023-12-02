//! Vec-like structure where indexes are kept for values even when removing others, usually used to
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
//! The operation [fixed_index_vec::FixedIndexVec::compress] would return
//! [(Index 7 changed to 1), (Index 6 changed to 5)] and the structure would look like this:
//! <br>
//! <br>
//! [ValueA, ValueH, ValueC, ValueD, ValueE, ValueG]
//! <br>
//! <br>
//! <br>
//! If you held the indexes in any  structure that can turn into an Iterator<&mut usize>, then the
//! result of the operation offers a [fixed_index_vec::compress_result::CompressResult] where every
//! pair of old and new index is found for every index that has changed.
//!
//! It also offers a function [fixed_index_vec::compress_result::CompressResult::update_old_indexes]
//! receiving an iterator of previous indexes and replaces them for the new ones.

#![no_std]
#![no_main]

extern crate alloc;

/// Defines the [fixed_index_vec::FixedIndexVec] structure and contents for their implementation
pub mod fixed_index_vec;