#![allow(incomplete_features)]
#![feature(const_generics)]

trait AddItem {
    type Item;
    type Index;
    fn add(&mut self, item: Self::Item) -> Self::Index;
}
impl<T> AddItem for Vec<T> {
    type Item = T;
    type Index = usize;
    fn add(&mut self, item: T) -> usize {
        self.push(item);
        self.len() - 1
    }
}
use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
pub struct MultiList<T, const S: usize> {
    store: RefCell<Vec<ListItem<T>>>,
    lists: [Cell<usize>; S],
    deleted: Cell<usize>,
}
impl<T, const S: usize> MultiList<T, S> {
    pub fn new() -> Self {
        MultiList {
            store: RefCell::new(Vec::default()),
            lists: unsafe { std::mem::transmute_copy(&[usize::INVALID; S]) },
            deleted: Cell::new(usize::INVALID),
        }
    }
    pub fn list_iter(&mut self, list_id: usize) -> ListIter<T, S> {
        assert!(list_id < S);
        ListIter {
            current: self.lists[list_id].get(),
            prev: usize::INVALID,
            list: self,
            first: &self.lists[list_id],
        }
    }
    pub fn push_front(&mut self, list_id: usize, item: T) {
        assert!(list_id < S);
        self.lists[list_id].set(self.store.borrow_mut().add(ListItem {
            next: self.lists[list_id].get(),
            data: item,
        }));
    }
    pub fn swap_lists(&mut self, list_id: usize, other_list_id: usize) {
        assert!(list_id < S);
        assert!(other_list_id < S);
        self.lists.swap(list_id, other_list_id);
    }
}
impl<T, const S: usize> Default for MultiList<T, S> {
    fn default() -> Self {
        MultiList::new()
    }
}

pub struct ListItem<T> {
    pub next: usize,
    pub data: T,
}

pub struct ListIter<'a, T, const S: usize> {
    list: &'a MultiList<T, S>,
    first: &'a Cell<usize>,
    current: usize,
    prev: usize,
}
impl<'a, T, const S: usize> ListIter<'a, T, S> {
    pub fn current<'b>(&'b mut self) -> Option<BorrowedListItem<'a, 'b, T, S>> {
        if self.current.is_valid() {
            Some(BorrowedListItem {
                item: unsafe {
                    self.list
                        .store
                        .borrow_mut()
                        .get_unchecked_mut(self.current)
                        .into()
                },
                iter: self,
            })
        } else {
            None
        }
    }
}

pub struct BorrowedListItem<'a, 'b, T, const S: usize> {
    item: NonNull<ListItem<T>>,
    iter: &'b mut ListIter<'a, T, S>,
}
impl<'a, 'b, T, const S: usize> BorrowedListItem<'a, 'b, T, S> {
    pub fn next(self) {
        self.iter.prev = self.iter.current;
        self.iter.current = unsafe { self.item.as_ref().next };
    }
    pub fn item(&mut self) -> &mut T {
        unsafe { &mut self.item.as_mut().data }
    }
    pub fn remove(mut self) {
        unsafe {
            let current = self.iter.current;
            let next = self.item.as_ref().next;
            let deleted = &self.iter.list.deleted;
            if deleted.get().is_valid() {
                self.item.as_mut().next = deleted.get();
            }
            deleted.set(current);
            if self.iter.first.get() == current {
                self.iter.first.set(next);
            } else {
                let prev = self.iter.prev;
                let mut store = self.iter.list.store.borrow_mut();
                let prev_item = store.get_unchecked_mut(prev);
                prev_item.next = next;
            }
        }
    }
    /// # Safety
    /// This function assumes the borrowed item is first in the list
    /// and skips the checks remove() usually performs
    pub unsafe fn remove_first(mut self) {
        let current = self.iter.current;
        let next = self.item.as_ref().next;
        let deleted = &self.iter.list.deleted;
        if deleted.get().is_valid() {
            self.item.as_mut().next = deleted.get();
        }
        deleted.set(current);
        self.iter.first.set(next);
    }
}

pub trait Index {
    const INVALID: usize;
    fn is_valid(&self) -> bool;
}
impl Index for usize {
    const INVALID: usize = usize::MAX;
    fn is_valid(&self) -> bool {
        *self != Self::INVALID
    }
}
