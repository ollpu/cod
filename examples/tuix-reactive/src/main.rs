
use tuix::*;
use cod::Node;

use std::rc::Rc;

#[derive(Node, Debug, Clone, PartialEq)]
struct Task {
    header: cod::Header,
    description: String,
    completed: bool,
}

#[derive(Node, Clone, Debug, Default)]
struct TodoState {
    header: cod::Header,
    tasks: Vec<cod::Child<Task>>,
}

#[derive(Debug, Clone, PartialEq)]
enum TodoEvent {
    Add(Option<Rc<Task>>),
    Remove,
    Debug,
}

#[derive(Debug, Clone)]
struct UpdateEvent<T>(Rc<T>);
impl<T> PartialEq for UpdateEvent<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

struct VecChildDiffHelper<T> {
    list: Vec<(cod::ID, Entity, *const T, bool)>
}
impl<T> Default for VecChildDiffHelper<T> {
    fn default() -> Self {
        Self { list: Default::default() }
    }
}

impl<T: Node + Clone> VecChildDiffHelper<T> where UpdateEvent<T>: Message {
    fn update<C: FnMut(&mut State, Rc<T>) -> Entity>(&mut self, state: &mut State, updated: &Vec<cod::Child<T>>, mut create: C) {
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

struct TodoApp {
    index: usize,
    states: Vec<cod::State<TodoState>>,

    data: Rc<TodoState>,
    tasks: VecChildDiffHelper<Task>,
}

impl TodoApp {
    pub fn new() -> Self {
        let mut states = Vec::new();

        // Add an initial state
        states.push(cod::State::new(&TodoState::default()));

        Self {
            index: 0,
            data: states.last().unwrap().root_ref(),
            states,
            tasks: Default::default(),
        }
    }

    pub fn get_current_state(&mut self) -> &mut cod::State<TodoState> {
        &mut self.states[self.index]
    }

    pub fn get_root(&self) -> Rc<TodoState> {
        self.states[self.index].root_ref()
    }
}

impl Widget for TodoApp {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        state.focused = entity;
        state.insert_event(Event::new(UpdateEvent(Rc::clone(&self.data))).target(entity));
        entity 
            .set_background_color(state, Color::blue())
            .set_flex_grow(state, 1.0)
    }

    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        // Handle Custom Todo Events
        if let Some(todo_event) = event.message.downcast::<TodoEvent>() {
            match todo_event {
                TodoEvent::Add(task) => {
                    println!("Add a Task");
                    
                    let header = self.get_current_state().root().header.clone();
                    let mut new_state = self.get_current_state().clone();
                    {
                        new_state.get_mut(new_state.root_ref()).tasks
                            .push(cod::Child::with_parent(&header, Task {
                                header: Default::default(),
                                description: "Test".to_string(),
                                completed: false,
                            }));                        
                    }
                    
                    state.insert_event(Event::new(UpdateEvent(new_state.root_ref())).target(entity));
                    self.states.push(new_state);
                    self.index += 1;
                }

                TodoEvent::Debug => {
                    println!("{:?}", self.get_current_state().root());
                }
                _ => {}
            }
        }
        
        // Handle Window Events
        if let Some(window_event) = event.message.downcast::<WindowEvent>() {
            match window_event {
                WindowEvent::KeyDown(code, _) => {
                    if *code == Code::KeyA {
                        // Send event to add new task
                        state.insert_event(Event::new(TodoEvent::Add(None)).target(entity));
                    }

                    if *code == Code::KeyD {
                        state.insert_event(Event::new(TodoEvent::Debug).target(entity));
                    }
                }
                _ => {}
            }
        }

        if let Some(UpdateEvent(new_node)) = event.message.downcast() {
            self.data = Rc::clone(new_node);
            self.tasks.update(state, &new_node.tasks, |state, child_ref| {
                TaskWidget::new(child_ref).build(state, entity, |builder| builder)
            });
        }
    }
}


struct TaskWidget {
    task: Rc<Task>
}

impl TaskWidget {
    pub fn new(task: Rc<Task>) -> Self {
        Self {
            task: task.clone(),
        }
    }
}

impl Widget for TaskWidget {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        state.insert_event(Event::new(UpdateEvent(Rc::clone(&self.task))).target(entity));
        entity
            .set_flex_basis(state, Length::Pixels(50.0))
            .set_background_color(state, Color::red())
            .set_margin(state, Length::Pixels(5.0))
    }
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(UpdateEvent(new_node)) = event.message.downcast() {
            self.task = Rc::clone(new_node);
            entity.set_text(state, &self.task.description);
        }
    }
}



fn main() {
    let app = Application::new(|state, window| {
        window.set_title("Tuix Todos");
        TodoApp::new().build(state, window.entity(), |builder| builder);
    });

    app.run();
}
