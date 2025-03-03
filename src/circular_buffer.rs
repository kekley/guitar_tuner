use std::{
    iter,
    mem::{self, MaybeUninit},
};

pub struct CircularBuffer<T: Sized> {
    size: usize,
    start: usize,
    items: Box<[MaybeUninit<T>]>,
}

impl<T> CircularBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let a = core::iter::repeat_with(MaybeUninit::uninit)
            .take(capacity)
            .collect::<Vec<_>>();

        Self {
            size: 0,
            start: 0,
            items: a.into_boxed_slice(),
        }
    }
    pub fn len(&self) -> usize {
        self.size
    }
    pub fn capacity(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    pub fn is_full(&self) -> bool {
        self.size == self.capacity()
    }

    pub fn make_contiguous(&mut self) -> &mut [T] {
        if self.capacity() == 0 || self.size == 0 {
            return &mut [];
        } else {
            debug_assert!(self.start < self.capacity(), "start out-of-bounds");
            debug_assert!(self.size <= self.capacity(), "size out-of-bounds");

            let start = self.start;
            let end = add_mod(start, self.size, self.capacity());

            let slice = if start < end {
                &mut self.items[start..end]
            } else {
                self.start = 0;
                self.items.rotate_left(start);
                &mut self.items[..self.size]
            };

            //SAFETY: indices guarantee values are initialized
            unsafe { slice_assume_init_mut(slice) }
        }
    }
    #[inline]
    fn front_maybe_uninit_mut(&mut self) -> &mut MaybeUninit<T> {
        debug_assert!(self.size > 0, "empty buffer");
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        &mut self.items[self.start]
    }

    #[inline]
    fn front_maybe_uninit(&self) -> &MaybeUninit<T> {
        debug_assert!(self.size > 0, "empty buffer");
        debug_assert!(self.size <= self.capacity(), "size out-of-bounds");
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        &self.items[self.start]
    }

    #[inline]
    fn back_maybe_uninit(&self) -> &MaybeUninit<T> {
        debug_assert!(self.size > 0, "empty buffer");
        debug_assert!(self.size <= self.capacity(), "size out-of-bounds");
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        let back = add_mod(self.start, self.size - 1, self.capacity());
        &self.items[back]
    }

    #[inline]
    fn back_maybe_uninit_mut(&mut self) -> &mut MaybeUninit<T> {
        debug_assert!(self.size > 0, "empty buffer");
        debug_assert!(self.size <= self.capacity(), "size out-of-bounds");
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        let back = add_mod(self.start, self.size - 1, self.capacity());
        &mut self.items[back]
    }

    #[inline]
    fn get_maybe_uninit(&self, index: usize) -> &MaybeUninit<T> {
        debug_assert!(self.size > 0, "empty buffer");
        debug_assert!(index < self.capacity(), "index out-of-bounds");
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        let index = add_mod(self.start, index, self.capacity());
        &self.items[index]
    }

    #[inline]
    fn get_maybe_uninit_mut(&mut self, index: usize) -> &mut MaybeUninit<T> {
        debug_assert!(self.size > 0, "empty buffer");
        debug_assert!(index < self.capacity(), "index out-of-bounds");
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        let index = add_mod(self.start, index, self.capacity());
        &mut self.items[index]
    }

    #[inline]
    fn slices_uninit_mut(&mut self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        if self.capacity() == 0 {
            return (&mut [][..], &mut [][..]);
        }

        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        debug_assert!(self.size <= self.capacity(), "size out-of-bounds");

        let start = self.start;
        let end = add_mod(start, self.size, self.capacity());
        if end < start {
            (&mut self.items[end..start], &mut [][..])
        } else {
            let (left, right) = self.items.split_at_mut(end);
            let left = &mut left[..start];
            (right, left)
        }
    }
    pub fn push_back(&mut self, item: T) -> Option<T> {
        if self.capacity() == 0 {
            return Some(item);
        } else {
            if self.size >= self.capacity() {
                let replaced_item = mem::replace(
                    unsafe { self.front_maybe_uninit_mut().assume_init_mut() },
                    item,
                );
                self.inc_start();
                Some(replaced_item)
            } else {
                self.inc_size();
                self.back_maybe_uninit_mut().write(item);
                None
            }
        }
    }
    #[inline]
    fn inc_start(&mut self) {
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        self.start = add_mod(self.start, 1, self.capacity());
    }

    #[inline]
    fn dec_start(&mut self) {
        debug_assert!(self.start < self.capacity(), "start out-of-bounds");
        self.start = sub_mod(self.start, 1, self.capacity());
    }

    #[inline]
    fn inc_size(&mut self) {
        debug_assert!(self.size <= self.capacity(), "size out-of-bounds");
        debug_assert!(self.size < self.capacity(), "size at capacity limit");
        self.size += 1;
    }

    #[inline]
    fn dec_size(&mut self) {
        debug_assert!(self.size > 0, "size is 0");
        self.size -= 1;
    }
}
#[inline]
const unsafe fn slice_assume_init_ref<T>(slice: &[MaybeUninit<T>]) -> &[T] {
    &*(slice as *const [MaybeUninit<T>] as *const [T])
}

#[inline]
unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
    &mut *(slice as *mut [MaybeUninit<T>] as *mut [T])
}
#[inline]
const fn add_mod(x: usize, y: usize, m: usize) -> usize {
    debug_assert!(m > 0);
    debug_assert!(x <= m);
    debug_assert!(y <= m);
    let (z, overflow) = x.overflowing_add(y);
    (z + (overflow as usize) * (usize::MAX % m + 1)) % m
}

#[inline]
const fn sub_mod(x: usize, y: usize, m: usize) -> usize {
    debug_assert!(m > 0);
    debug_assert!(x <= m);
    debug_assert!(y <= m);
    add_mod(x, m - y, m)
}
