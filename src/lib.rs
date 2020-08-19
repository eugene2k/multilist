#![feature(untagged_unions)]
#![feature(const_generics)]
use std::mem::ManuallyDrop;

#[derive(Clone, Copy)]
pub struct Index {
    inner: usize,
}
impl Index {
    fn new_invalid() -> Self {
        Self { inner: usize::MAX }
    }
    pub fn is_valid(&self) -> bool {
        self.inner != usize::MAX
    }
}
union MaybeFree<T, I: Copy> {
    next_free: I,
    occupied: ManuallyDrop<T>,
}
pub struct ListItem<T> {
    next: Index,
    prev: Index,
    pub payload: T,
}
impl<T> ListItem<T> {
    pub fn next(&self) -> Index {
        self.next
    }
    pub fn prev(&self) -> Index {
        self.prev
    }
}

pub struct InnerVec<T> {
    inner: Vec<T>,
}
impl<T> InnerVec<T> {
    fn new() -> Self {
        InnerVec { inner: Vec::new() }
    }
    fn get(&self, index: Index) -> Option<&T> {
        self.inner.get(index.inner)
    }
    fn get_mut(&mut self, index: Index) -> Option<&mut T> {
        self.inner.get_mut(index.inner)
    }
    unsafe fn get_unchecked(&self, index: Index) -> &T {
        self.inner.get_unchecked(index.inner)
    }
    unsafe fn get_unchecked_mut(&mut self, index: Index) -> &mut T {
        self.inner.get_unchecked_mut(index.inner)
    }
    unsafe fn get_unchecked_mut_ptr(&mut self, index: Index) -> *mut T {
        self.inner.get_unchecked_mut(index.inner)
    }
    fn push(&mut self, item: T) {
        self.inner.push(item)
    }
    fn len(&self) -> usize {
        self.inner.len()
    }
}

pub struct ListStore<T> {
    buffer: InnerVec<MaybeFree<ListItem<T>, Index>>,
    last_free: Index,
}
impl<T> ListStore<T> {
    fn new() -> Self {
        ListStore {
            buffer: InnerVec::new(),
            last_free: Index::new_invalid(),
        }
    }
    unsafe fn remove(&mut self, index: Index) {
        let buffer = &mut self.buffer;
        let item = &mut *buffer.get_unchecked_mut_ptr(index); // real index should be index - 1
        if item.occupied.next.is_valid() {
            let next_item = buffer.get_unchecked_mut(item.occupied.next);
            next_item.occupied.prev = item.occupied.prev;
        }
        if item.occupied.prev.is_valid() {
            let prev_item = buffer.get_unchecked_mut(item.occupied.prev);
            prev_item.occupied.next = item.occupied.next;
        }
        ManuallyDrop::drop(&mut item.occupied);
        item.next_free = self.last_free;
        self.last_free = index;
    }
    unsafe fn get_mut(&mut self, index: Index) -> &mut ListItem<T> {
        &mut self.buffer.get_unchecked_mut(index).occupied
    }
    unsafe fn add(&mut self, item: ListItem<T>) -> Index {
        let buffer = &mut self.buffer;
        let item = MaybeFree {
            occupied: ManuallyDrop::new(item),
        };
        if self.last_free.is_valid() {
            let place = buffer.get_unchecked_mut(self.last_free);
            std::ptr::write(place, item);
            self.last_free
        } else {
            buffer.push(item);
            Index {
                inner: buffer.len(),
            }
        }
    }
}
#[derive(Clone, Copy)]
struct ListPointers {
    head: Index,
    tail: Index,
}
impl ListPointers {
    fn new_invalid() -> Self {
        Self {
            head: Index::new_invalid(),
            tail: Index::new_invalid(),
        }
    }
}
pub struct MultiList<T, const S: usize> {
    store: ListStore<T>,
    lists: [ListPointers; S],
}
impl<T, const S: usize> MultiList<T, S> {
    pub fn new() -> Self {
        MultiList {
            store: ListStore::new(),
            lists: [ListPointers::new_invalid(); S],
        }
    }
    pub unsafe fn first(&self, index: usize) -> Index {
        self.lists.get_unchecked(index).head
    }
    pub fn push_back(&mut self, group: usize, item: T) -> Index {
        let ListPointers { head, tail: prev } = self.lists[group as usize];
        let item = ListItem {
            next: Index::new_invalid(),
            prev,
            payload: item,
        };
        let index = unsafe { self.store.add(item) };
        if head.is_valid() {
            self.lists[group as usize] = ListPointers { head, tail: index };
        } else {
            self.lists[group as usize] = ListPointers {
                head: index,
                tail: index,
            };
        }
        index
    }
    pub fn get_mut(&mut self, index: Index) -> &mut ListItem<T> {
        unsafe { self.store.get_mut(index) }
    }
    pub fn remove(&mut self, index: Index) {
        unsafe { self.store.remove(index) }
    }
}
impl<T, const S: usize> std::default::Default for MultiList<T, S> {
    fn default() -> Self {
        MultiList::new()
    }
}
#[derive(Clone, Copy)]
pub enum Group {
    One = 0,
    Two = 1,
    Three = 2,
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
