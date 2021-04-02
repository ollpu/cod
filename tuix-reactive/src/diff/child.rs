
use tuix::*;
use cod::Node;
use cod::Rc;

use crate::UpdateEvent;
use super::option::OptionDiffer;

/// TODO: add ConstChildDiffer where the child widget is never rebuilt (when does
/// that make sense?)
#[derive(Default)]
pub struct ChildDiffer<T>(OptionDiffer<T>);

impl<T: Node + Clone> ChildDiffer<T> where UpdateEvent<T>: Message {
    pub fn update<C: FnMut(&mut State, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: &cod::Child<T>, create: C) {
        self.0.update_raw(state, Some(updated), create);
    }
}
