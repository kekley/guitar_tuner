// Copyright © 2023-2025 Andrea Corbellini and contributors
// SPDX-License-Identifier: BSD-3-Clause

use crate::circular_buffer;
use crate::circular_buffer::CircularBuffer;
use core::fmt;
use core::iter::FusedIterator;
use core::ops::Bound;
use core::ops::RangeBounds;
use std::mem::MaybeUninit;

/// An owning [iterator](core::iter::Iterator) over the elements of a [`CircularBuffer`].
///
/// This yields the elements of a `CircularBuffer` from front to back.
///
/// This struct is created when iterating over a `CircularBuffer`. See the documentation for
/// [`IntoIterator`] for more details.
#[derive(Clone)]
pub struct IntoIter<T> {
    inner: CircularBuffer<T>,
}

impl<T> IntoIter<T> {
    pub(crate) const fn new(inner: CircularBuffer<T>) -> Self {
        Self { inner }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop_front()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.len();
        (len, Some(len))
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T> FusedIterator for IntoIter<T> where Box<[MaybeUninit<T>]>: Clone {}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.pop_back()
    }
}

impl<T> fmt::Debug for IntoIter<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

pub(crate) fn translate_range_bounds<T, R>(buf: &CircularBuffer<T>, range: R) -> (usize, usize)
where
    R: RangeBounds<usize>,
{
    let start = match range.start_bound() {
        Bound::Included(x) => *x,
        Bound::Excluded(x) => x
            .checked_add(1)
            .expect("range start index exceeds maximum usize"),
        Bound::Unbounded => 0,
    };

    let end = match range.end_bound() {
        Bound::Included(x) => x
            .checked_add(1)
            .expect("range end index exceeds maximum usize"),
        Bound::Excluded(x) => *x,
        Bound::Unbounded => buf.len(),
    };

    assert!(
        end <= buf.len(),
        "range end index {} out of range for buffer of length {}",
        end,
        buf.len()
    );
    assert!(
        start <= end,
        "range starts at index {start} but ends at index {end}"
    );

    (start, end)
}

#[cfg(not(feature = "unstable"))]
fn slice_take<'a, T, R: RangeBounds<usize>>(slice: &mut &'a [T], range: R) -> Option<&'a [T]> {
    match (range.start_bound(), range.end_bound()) {
        (Bound::Unbounded, Bound::Excluded(index)) => {
            if *index > slice.len() {
                return None;
            }
            let (left, right) = slice.split_at(*index);
            *slice = right;
            Some(left)
        }
        (Bound::Included(index), Bound::Unbounded) => {
            if *index > slice.len() {
                return None;
            }
            let (left, right) = slice.split_at(*index);
            *slice = left;
            Some(right)
        }
        _ => unimplemented!(),
    }
}

#[cfg(not(feature = "unstable"))]
fn slice_take_mut<'a, T, R: RangeBounds<usize>>(
    slice: &mut &'a mut [T],
    range: R,
) -> Option<&'a mut [T]> {
    match (range.start_bound(), range.end_bound()) {
        (Bound::Unbounded, Bound::Excluded(index)) => {
            if *index > slice.len() {
                return None;
            }
            let (left, right) = core::mem::take(slice).split_at_mut(*index);
            *slice = right;
            Some(left)
        }
        (Bound::Included(index), Bound::Unbounded) => {
            if *index > slice.len() {
                return None;
            }
            let (left, right) = core::mem::take(slice).split_at_mut(*index);
            *slice = left;
            Some(right)
        }
        _ => unimplemented!(),
    }
}

#[cfg(not(feature = "unstable"))]
fn slice_take_first<'a, T>(slice: &mut &'a [T]) -> Option<&'a T> {
    let (item, rest) = slice.split_first()?;
    *slice = rest;
    Some(item)
}

#[cfg(not(feature = "unstable"))]
fn slice_take_first_mut<'a, T>(slice: &mut &'a mut [T]) -> Option<&'a mut T> {
    let (item, rest) = core::mem::take(slice).split_first_mut()?;
    *slice = rest;
    Some(item)
}

#[cfg(not(feature = "unstable"))]
fn slice_take_last<'a, T>(slice: &mut &'a [T]) -> Option<&'a T> {
    let (item, rest) = slice.split_last()?;
    *slice = rest;
    Some(item)
}

#[cfg(not(feature = "unstable"))]
fn slice_take_last_mut<'a, T>(slice: &mut &'a mut [T]) -> Option<&'a mut T> {
    let (item, rest) = core::mem::take(slice).split_last_mut()?;
    *slice = rest;
    Some(item)
}

/// An [iterator](core::iter::Iterator) over the elements of a `CircularBuffer`.
///
/// This struct is created by [`CircularBuffer::iter()`] and [`CircularBuffer::range()`]. See
/// their documentation for more details.
pub struct Iter<'a, T> {
    pub(crate) right: &'a [T],
    pub(crate) left: &'a [T],
}

impl<'a, T> Iter<'a, T> {
    pub(crate) const fn empty() -> Self {
        Self {
            right: &[],
            left: &[],
        }
    }

    pub(crate) fn new(buf: &'a CircularBuffer<T>) -> Self
where {
        let (right, left) = buf.as_slices();
        Self { right, left }
    }

    pub(crate) fn over_range<R>(buf: &'a CircularBuffer<T>, range: R) -> Self
    where
        R: RangeBounds<usize>,
    {
        let (start, end) = translate_range_bounds(buf, range);
        if start >= end {
            Self::empty()
        } else {
            let len = buf.len();
            let mut it = Self::new(buf);
            it.advance_front_by(start);
            it.advance_back_by(len - end);
            it
        }
    }

    fn advance_front_by(&mut self, count: usize) {
        if self.right.len() > count {
            slice_take(&mut self.right, ..count);
        } else {
            let take_left = count - self.right.len();
            debug_assert!(
                take_left <= self.left.len(),
                "attempted to advance past the back of the buffer"
            );
            slice_take(&mut self.left, ..take_left);
            self.right = &[];
        }
    }

    fn advance_back_by(&mut self, count: usize) {
        if self.left.len() > count {
            let take_left = self.left.len() - count;
            slice_take(&mut self.left, take_left..);
        } else {
            let take_right = self.right.len() - (count - self.left.len());
            debug_assert!(
                take_right <= self.right.len(),
                "attempted to advance past the front of the buffer"
            );
            slice_take(&mut self.right, take_right..);
            self.left = &[];
        }
    }
}

impl<T> Default for Iter<'_, T> {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = slice_take_first(&mut self.right) {
            Some(item)
        } else if let Some(item) = slice_take_first(&mut self.left) {
            Some(item)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        self.right.len() + self.left.len()
    }
}

impl<T> FusedIterator for Iter<'_, T> {}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(item) = slice_take_last(&mut self.left) {
            Some(item)
        } else if let Some(item) = slice_take_last(&mut self.right) {
            Some(item)
        } else {
            None
        }
    }
}

impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            right: self.right,
            left: self.left,
        }
    }
}

impl<T> fmt::Debug for Iter<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A mutable [iterator](core::iter::Iterator) over the elements of a `CircularBuffer`.
///
/// This struct is created by [`CircularBuffer::iter_mut()`] and [`CircularBuffer::range_mut()`].
/// See their documentation for more details.
pub struct IterMut<'a, T> {
    right: &'a mut [T],
    left: &'a mut [T],
}

impl<'a, T> IterMut<'a, T> {
    pub(crate) fn empty() -> Self {
        Self {
            right: &mut [],
            left: &mut [],
        }
    }

    pub(crate) fn new(buf: &'a mut CircularBuffer<T>) -> Self
where {
        let (right, left) = buf.as_mut_slices();
        Self { right, left }
    }

    pub(crate) fn over_range<R>(buf: &'a mut CircularBuffer<T>, range: R) -> Self
    where
        R: RangeBounds<usize>,
    {
        let (start, end) = translate_range_bounds(buf, range);
        if start >= end {
            Self::empty()
        } else {
            let len = buf.len();
            let mut it = Self::new(buf);
            it.advance_front_by(start);
            it.advance_back_by(len - end);
            it
        }
    }

    fn advance_front_by(&mut self, count: usize) {
        if self.right.len() > count {
            slice_take_mut(&mut self.right, ..count);
        } else {
            let take_left = count - self.right.len();
            debug_assert!(
                take_left <= self.left.len(),
                "attempted to advance past the back of the buffer"
            );
            slice_take_mut(&mut self.left, ..take_left);
            self.right = &mut [];
        }
    }

    fn advance_back_by(&mut self, count: usize) {
        if self.left.len() > count {
            let take_left = self.left.len() - count;
            slice_take_mut(&mut self.left, take_left..);
        } else {
            let take_right = self.right.len() - (count - self.left.len());
            debug_assert!(
                take_right <= self.right.len(),
                "attempted to advance past the front of the buffer"
            );
            slice_take_mut(&mut self.right, take_right..);
            self.left = &mut [];
        }
    }
}

impl<T> Default for IterMut<'_, T> {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = slice_take_first_mut(&mut self.right) {
            Some(item)
        } else if let Some(item) = slice_take_first_mut(&mut self.left) {
            Some(item)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        self.right.len() + self.left.len()
    }
}

impl<T> FusedIterator for IterMut<'_, T> {}

impl<T> DoubleEndedIterator for IterMut<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(item) = slice_take_last_mut(&mut self.left) {
            Some(item)
        } else if let Some(item) = slice_take_last_mut(&mut self.right) {
            Some(item)
        } else {
            None
        }
    }
}

impl<T> fmt::Debug for IterMut<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let it = Iter {
            right: self.right,
            left: self.left,
        };
        it.fmt(f)
    }
}
