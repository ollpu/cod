
use tuix::*;
use cod::Node;
use cod::Rc;

use crate::{DynUpdateEvent, AnimationRequest};

pub struct OptionDiffer<T> {
    container: Entity,
    child: Option<(cod::ID, Entity, *const T)>,
}
impl<T> Default for OptionDiffer<T> {
    fn default() -> Self {
        Self { 
            container: Entity::null(),
            child: None,
        }
    }
}

impl<T: Node + Clone> OptionDiffer<T> {
    pub fn set_container(&mut self, entity: Entity) {
        self.container = entity;
    }
    pub fn update<C: FnMut(&mut State, Entity, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: &Option<cod::Child<T>>, animate: bool, create: C) {
        self.update_raw(state, updated.as_ref(), animate, create);
    }

    pub fn update_raw<C: FnMut(&mut State, Entity, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: Option<&cod::Child<T>>, animate: bool, mut create: C) {
        match (&mut self.child, updated) {
            (Some(ref mut cur), Some(upd)) if cur.0 == upd.header().id() => {
                let node = upd.get_ref();
                if cur.2 != Rc::as_ptr(&node) {
                    cur.2 = Rc::as_ptr(&node);
                    state.insert_event(Event::new(DynUpdateEvent::Update(node, animate)).direct(cur.1));
                }
            },
            _ => {
                // works by the following logic: if animate == true, then
                // (Some, Some) => replace without animating
                // (Some, None) => remove and animate
                // (None, Some) => add and animate
                if let Some(cur) = self.child {
                    if animate && updated.is_none() {
                        state.insert_event(Event::new(DynUpdateEvent::Remove(cur.0, true)).direct(cur.1));
                    } else {
                        state.remove(cur.1);
                    }
                }
                self.child = if let Some(upd) = updated {
                    let upd_ref = upd.get_ref();
                    let entity = create(state, self.container, upd_ref.clone());
                    if animate && self.child.is_none() {
                        state.insert_event(Event::new(AnimationRequest::Appear).direct(entity));
                    }
                    Some((upd_ref.header().id(), entity, Rc::as_ptr(&upd_ref)))
                } else {
                    None
                }
            }
        }
    }
}
