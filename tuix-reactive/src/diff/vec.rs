use tuix::*;
use cod::Node;
use cod::Rc;

use crate::UpdateEvent;

pub struct VecDiffer<T> {
    list: Vec<(cod::ID, Entity, *const T, bool)>
}
impl<T> Default for VecDiffer<T> {
    fn default() -> Self {
        Self { list: Default::default() }
    }
}

impl<T: Node + Clone> VecDiffer<T> where UpdateEvent<T>: Message {
    pub fn update<C: FnMut(&mut State, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: &Vec<cod::Child<T>>, mut create: C) {
        let fast_path = self.list.iter().map(|it| it.0).eq(
            // TODO: potential optimization by caching ID in Child
            updated.iter().map(|ch| ch.get_id())
        );
        if fast_path {
            for (old, upd) in self.list.iter().zip(updated.iter()) {
                let upd_ref = upd.get_ref();
                if old.2 != Rc::as_ptr(&upd_ref) {
                    state.insert_event(Event::new(UpdateEvent(upd_ref)).target(old.1));
                }
            }
        } else {
            self.list.sort_unstable_by_key(|it| it.0);
            debug_assert!(self.list.iter().all(|it| !it.3));
            let mut new_list = Vec::new();
            for upd in updated.iter() {
                let upd_ref = upd.get_ref();
                match self.list.binary_search_by_key(&upd.get_id(), |it| it.0) {
                    Ok(i) => {
                        let ref mut old = self.list[i];
                        new_list.push((old.0, old.1, Rc::as_ptr(&upd_ref), false));
                        state.insert_event(Event::new(UpdateEvent(upd_ref)).target(old.1));
                        old.3 = true;
                    },
                    Err(_) => {
                       let entity = create(state, upd_ref.clone());
                       new_list.push((upd.get_id(), entity, Rc::as_ptr(&upd_ref), false));
                    }
                }
            }
            for it in self.list.iter() {
                if !it.3 {
                    state.remove(it.1);
                }
            }
            self.list = new_list;
            for (l, r) in self.list.iter().zip(self.list.iter().skip(1)) {
                // FIXME: Returns HierarchyError when already adjacent, needs distinction
                let _ = state.hierarchy.set_next_sibling(l.1, r.1);
            }
        }
    }
}
