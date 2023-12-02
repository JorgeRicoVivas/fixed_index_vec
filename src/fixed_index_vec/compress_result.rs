use alloc::vec::Vec;

/// Holds the result of executing [super::FixedIndexVec::compress], which is a Vector containing
/// every index that has changed along its new value.
pub struct CompressResult(pub Vec<(usize, usize)>);

impl CompressResult {
    /// Replaces all values on in this iterator that matches to an the old indexes with the new
    /// indexes.
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
