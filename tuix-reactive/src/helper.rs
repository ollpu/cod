use tuix::*;
use cod::NodeClone;
use crate::{UpdateEvent, DynUpdateEvent};
use cod::{Rc, downcast_rc};

/// Should not be used when widget can receive different types of update events,
/// because `Remove`s are ambiguous.
pub fn downcast_update<T: NodeClone>(event: &mut Event) -> Option<UpdateEvent<T>> {
    let update = event.message.downcast::<DynUpdateEvent>()?;
    Some(match update {
        UpdateEvent::Update(node, animate) => {
            let node = downcast_rc(Rc::clone(node))?;
            UpdateEvent::Update(node, *animate)
        }
        UpdateEvent::Remove(id, animate) => UpdateEvent::Remove(*id, *animate),
    })
}

pub fn initial_update<W: Widget, T: NodeClone>(widget: &mut W, state: &mut State, entity: Entity, data: Rc<T>) {
    // just send an event instead? this way the init is done immediately, which might be necessary
    // in some use-cases..?
    widget.on_event(state, entity, &mut Event::new(UpdateEvent::Update(data, false).into_dyn()));
}
