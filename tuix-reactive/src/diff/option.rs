
use tuix::*;
use cod::Node;
use cod::Rc;

use crate::UpdateEvent;

pub struct OptionDiffer<T> {
    child: Option<(cod::ID, Entity, *const T)>,
}
impl<T> Default for OptionDiffer<T> {
    fn default() -> Self {
        Self { child: None }
    }
}

impl<T: Node + Clone> OptionDiffer<T> where UpdateEvent<T>: Message {
    pub fn update<C: FnMut(&mut State, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: &Option<cod::Child<T>>, create: C) {
        self.update_raw(state, updated.as_ref(), create);
    }

    pub(crate) fn update_raw<C: FnMut(&mut State, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: Option<&cod::Child<T>>, mut create: C) {
        match (&mut self.child, updated) {
            (Some(ref mut cur), Some(upd)) if cur.0 == upd.header().id() => {
                let node = upd.get_ref();
                if cur.2 != Rc::as_ptr(&node) {
                    cur.2 = Rc::as_ptr(&node);
                    state.insert_event(Event::new(UpdateEvent(node)).target(cur.1));
                }
            },
            _ => {
                if let Some(cur) = self.child {
                    state.remove(cur.1);
                }
                self.child = if let Some(upd) = updated {
                    let upd_ref = upd.get_ref();
                    let entity = create(state, upd_ref.clone());
                    Some((upd_ref.header().id(), entity, Rc::as_ptr(&upd_ref)))
                } else {
                    None
                }
            }
        }
    }
}
