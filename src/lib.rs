#![feature(untagged_unions)]
#![feature(const_generics)]

use std::mem::ManuallyDrop;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Index {
    inner: usize,
}
impl Index {
    pub fn new_invalid() -> Self {
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
    unsafe fn extend(&mut self, additional: usize) {
        self.inner.reserve(additional);
        self.inner.set_len(additional);
    }
    fn len(&self) -> usize {
        self.inner.len()
    }
}

pub struct List<T> {
    buffer: InnerVec<MaybeFree<ListItem<T>, Index>>,
    last_free: Index,
    head: Index,
    tail: Index,
}
impl<T> List<T> {
    pub fn new() -> Self {
        List {
            buffer: InnerVec::new(),
            last_free: Index::new_invalid(),
            head: Index::new_invalid(),
            tail: Index::new_invalid(),
        }
    }
    pub fn head(&self) -> Index {
        self.head
    }
    pub fn tail(&self) -> Index {
        self.tail
    }
    pub fn push_front(&mut self, item: T) -> Index {
        let index = {
            let buffer = &mut self.buffer;
            let item = MaybeFree {
                occupied: ManuallyDrop::new(ListItem {
                    next: self.head,
                    prev: Index::new_invalid(),
                    payload: item,
                }),
            };
            if self.last_free.is_valid() {
                unsafe {
                    let place = buffer.get_unchecked_mut(self.last_free);
                    std::ptr::write(place, item);
                }
                self.last_free
            } else {
                buffer.push(item);
                Index {
                    inner: buffer.len(),
                }
            }
        };
        if !self.tail.is_valid() {
            self.tail = index
        }
        self.head = index;
        index
    }
    pub fn push_back(&mut self, item: T) -> Index {
        let index = {
            let buffer = &mut self.buffer;
            let item = MaybeFree {
                occupied: ManuallyDrop::new(ListItem {
                    next: Index::new_invalid(),
                    prev: self.tail,
                    payload: item,
                }),
            };
            if self.last_free.is_valid() {
                unsafe {
                    let place = buffer.get_unchecked_mut(self.last_free);
                    std::ptr::write(place, item);
                }
                self.last_free
            } else {
                buffer.push(item);
                Index {
                    inner: buffer.len(),
                }
            }
        };
        if !self.head.is_valid() {
            self.head = index
        }
        self.tail = index;
        index
    }
    fn future_index(&mut self) -> Index {
        if self.last_free.is_valid() {
            let index = self.last_free;
            self.last_free = unsafe { self.buffer.get_unchecked(index).next_free };
            index
        } else {
            let index = Index {
                inner: self.buffer.len(),
            };
            unsafe { self.buffer.extend(index.inner + 1) };
            index
        }
    }
    pub fn insert_before(&mut self, index: Index, item: T) {
        let future_index = self.future_index();
        unsafe {
            let insert_before_item = self.buffer.get_unchecked_mut(index);
            insert_before_item.occupied.prev = future_index;
            let prev_item_index = insert_before_item.occupied.prev;

            let place = self.buffer.get_unchecked_mut(future_index);

            std::ptr::write(
                place,
                MaybeFree {
                    occupied: ManuallyDrop::new(ListItem {
                        prev: prev_item_index,
                        next: index,
                        payload: item,
                    }),
                },
            );
            let prev_item = self.buffer.get_unchecked_mut(prev_item_index);
            prev_item.occupied.next = future_index;
        }
    }
    pub fn insert_after(&mut self, index: Index, item: T) {
        let future_index = self.future_index();
        unsafe {
            let insert_after_item = self.buffer.get_unchecked_mut(index);
            insert_after_item.occupied.next = future_index;
            let next_item_index = insert_after_item.occupied.next;

            let place = self.buffer.get_unchecked_mut(future_index);

            std::ptr::write(
                place,
                MaybeFree {
                    occupied: ManuallyDrop::new(ListItem {
                        prev: index,
                        next: next_item_index,
                        payload: item,
                    }),
                },
            );
            let next_item = self.buffer.get_unchecked_mut(next_item_index);
            next_item.occupied.prev = future_index;
        }
    }
    pub fn get_mut(&mut self, index: Index) -> &mut ListItem<T> {
        unsafe { &mut self.buffer.get_unchecked_mut(index).occupied }
    }
    pub fn get_unchecked(&mut self, index: Index) -> &ListItem<T> {
        unsafe { &self.buffer.get_unchecked(index).occupied }
    }
    pub fn remove(&mut self, index: Index) {
        unsafe {
            let buffer = &mut self.buffer;
            let item = &mut *buffer.get_unchecked_mut_ptr(index);
            if item.occupied.next.is_valid() {
                let next_item = buffer.get_unchecked_mut(item.occupied.next);
                next_item.occupied.prev = item.occupied.prev;
            } else {
                self.tail = item.occupied.prev;
            }
            if item.occupied.prev.is_valid() {
                let prev_item = buffer.get_unchecked_mut(item.occupied.prev);
                prev_item.occupied.next = item.occupied.next;
            } else {
                self.head = item.occupied.next;
            }
            ManuallyDrop::drop(&mut item.occupied);
            item.next_free = self.last_free;
            self.last_free = index;
        }
    }
}
impl<T> std::default::Default for List<T> {
    fn default() -> Self {
        List::new()
    }
}
/*
struct MultiList<T, const LISTS: usize> {
    buffer: InnerVec<MaybeFree<ListItem<T>, Index>>,
    last_free: Index,
    heads: [Index; LISTS],
    tail: Index,
}
impl<T, const LISTS: usize> MultiList<T, LISTS> {
    fn new() -> Self {
        MultiList {
            buffer: InnerVec::new(),
            last_free: Index::new_invalid(),
            heads: [Index::new_invalid(); LISTS],
            tail: Index::new_invalid(),
        }
    }
    fn push_back(&mut self, list: usize, item: T) {
        // FIXME: what if the separator_head is invalid?
    }
    fn remove(&mut self, list: usize, index: Index) {
        if list > 0 && self.heads[list - 1] == index {
            self.heads[list - 1] = Index::new_invalid();
        }
        self.list.remove(index);
    }
}
*/
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
    buffer: InnerVec<MaybeFree<ListItem<T>, Index>>,
    last_free: Index,
    lists: [ListPointers; S],
}
impl<T, const S: usize> MultiList<T, S> {
    pub fn new() -> Self {
        MultiList {
            buffer: InnerVec::new(),
            last_free: Index::new_invalid(),
            lists: [ListPointers::new_invalid(); S],
        }
    }
    pub unsafe fn head_unchecked(&self, index: usize) -> Index {
        self.lists.get_unchecked(index).head
    }
    pub fn push_back(&mut self, list: usize, item: T) -> Index {
        let ListPointers { head, tail: prev } = self.lists[list as usize];
        let index = {
            let item = MaybeFree {
                occupied: ManuallyDrop::new(ListItem {
                    next: Index::new_invalid(),
                    prev,
                    payload: item,
                }),
            };
            if self.last_free.is_valid() {
                let retval = self.last_free;
                self.last_free = unsafe {
                    let free_item = self.buffer.get_unchecked_mut(retval);
                    std::ptr::write(free_item, item);
                    free_item.next_free
                };
                retval
            } else {
                self.buffer.push(item);
                Index {
                    inner: self.buffer.len() - 1,
                }
            }
        };
        if head.is_valid() {
            self.lists[list as usize] = ListPointers { head, tail: index };
        } else {
            self.lists[list as usize] = ListPointers {
                head: index,
                tail: index,
            };
        }
        index
    }
    /// # Safety
    /// This function expects the index to be valid. Result is undefined otherwise.
    pub unsafe fn get_unchecked_mut(&mut self, index: Index) -> &mut ListItem<T> {
        &mut self.buffer.get_unchecked_mut(index).occupied
    }
    /// # Safety
    /// This function doesn't check if the list exists and expects the list
    /// to not be empty (head index must be valid). Otherwise the result is undefined.
    pub unsafe fn remove_first_unchecked(&mut self, list: usize) {
        let head = self.head_unchecked(list);
        let item = self.buffer.get_unchecked_mut(head);
        ManuallyDrop::drop(&mut item.occupied);
        item.next_free = self.last_free;
        self.last_free = head;
    }
    pub fn remove(&mut self, list: usize, index: Index) {
        unsafe {
            let buffer = &mut self.buffer;
            let item = &mut *buffer.get_unchecked_mut_ptr(index);
            if item.occupied.next.is_valid() {
                let next_item = buffer.get_unchecked_mut(item.occupied.next);
                next_item.occupied.prev = item.occupied.prev;
            } else {
                self.lists[list].tail = item.occupied.prev;
            }
            if item.occupied.prev.is_valid() {
                let prev_item = buffer.get_unchecked_mut(item.occupied.prev);
                prev_item.occupied.next = item.occupied.next;
            } else {
                self.lists[list].head = item.occupied.next;
            }
            ManuallyDrop::drop(&mut item.occupied);
            item.next_free = self.last_free;
            self.last_free = index;
        }
    }
}
impl<T, const S: usize> std::default::Default for MultiList<T, S> {
    fn default() -> Self {
        MultiList::new()
    }
}

#[cfg(test)]
mod test {
    use super::{Index, MultiList};
    #[test]
    fn test_multilist_remove_invalid_from_empty() {
        let mut multilist = MultiList::<u8, 5>::new();
        multilist.remove(0, Index::new_invalid())
    }
    #[test]
    fn test_multilist_push_back() {
        let mut multilist = MultiList::<_, 5>::new();
        let item_id = multilist.push_back(0, "Test".to_string());
        let item = unsafe { multilist.get_unchecked_mut(item_id) };
        assert_eq!(item.payload, "Test".to_string());
    }
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
