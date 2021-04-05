use tuix::*;
use cod::NodeClone;
use crate::{MutationManager, MutationEvent};

use std::collections::VecDeque;

/// Basic undo manager with undo & redo stacks. All "future" redo states are forgotten
/// when a new change is made.
pub struct BasicUndoManager<T: NodeClone + Clone> {
    mutation_manager: MutationManager<T>,
    state_active: bool,
    undo_stack: VecDeque<cod::State<T>>,
    redo_stack: Vec<cod::State<T>>,
    undo_limit: usize,
}

impl<T: NodeClone + Clone> BasicUndoManager<T> {
    pub fn new(state: cod::State<T>, undo_limit: usize) -> Self {
        Self {
            mutation_manager: MutationManager::new(state),
            state_active: false,
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            undo_limit,
        }
    }

    pub fn on_event(&mut self, state: &mut State, event: &mut Event) {
        if !self.state_active {
            if let Some(_) = event.message.downcast::<MutationEvent>() {
                // State was inactive (view-only), but a mutation was received.
                self.start_undo_state();
            }
        }
        self.mutation_manager.on_event(state, event);
    }

    pub fn start_undo_state(&mut self) {
        if self.undo_stack.len() >= self.undo_limit {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(self.mutation_manager.clone_state());
        self.redo_stack.clear();
        self.state_active = true;
    }

    pub fn undo(&mut self, state: &mut State) {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push(self.mutation_manager.clone_state());
            self.mutation_manager.replace_state(state, prev);
        }
        self.state_active = false;
    }

    pub fn redo(&mut self, state: &mut State) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push_back(self.mutation_manager.clone_state());
            self.mutation_manager.replace_state(state, next);
        }
        self.state_active = false;
    }
}

