
use tuix::*;
use cod::Node;
use cod::Rc;

use super::option::OptionDiffer;

/// TODO: add ConstChildDiffer where the child widget is never rebuilt (when does
/// that make sense?)
#[derive(Default)]
pub struct ChildDiffer<T>(OptionDiffer<T>);

impl<T: Node + Clone> ChildDiffer<T> {
    pub fn set_container(&mut self, entity: Entity) {
        self.0.set_container(entity);
    }
    pub fn update<C: FnMut(&mut State, Entity, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: &cod::Child<T>, animate: bool, create: C) {
        self.0.update_raw(state, Some(updated), animate, create);
    }
}
