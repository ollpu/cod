
use tuix::*;
use tuix_reactive::*;
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

struct TodoApp {
    mutation_manager: MutationManager<TodoState>,

    data: Rc<TodoState>,
    tasks: VecDiffer<Task>,
}

impl TodoApp {
    pub fn new() -> Self {
        //let mut states = Vec::new();

        // Add an initial state
        let state = cod::State::new(&TodoState::default());

        Self {
            //index: 0,
            data: state.root_ref(),
            mutation_manager: MutationManager::new(state),
            //states,
            tasks: Default::default(),
        }
    }
}

impl Widget for TodoApp {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        state.focused = entity;
        configure_observer(state, entity, ConfigureObserver::RegisterRoot);
        entity 
            .set_background_color(state, Color::blue())
            .set_flex_grow(state, 1.0)
    }

    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        self.mutation_manager.on_event(state, event);
        // Handle Custom Todo Events
        if let Some(todo_event) = event.message.downcast() {
            match todo_event {
                TodoEvent::Add(task) => {
                    println!("Add a Task");
                    
                    mutate(state, entity, &self.data, |data| {
                        data.tasks.push(cod::Child::with_parent(&*data, Task {
                            header: Default::default(),
                            description: "Test".to_owned(),
                            completed: false,
                        }));
                    });
                }

                TodoEvent::Debug => {
                    println!("{:?}", self.data);
                }
                _ => {}
            }
        }
        
        // Handle Window Events
        if let Some(window_event) = event.message.downcast() {
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

        if let Some(observation) = event.message.downcast() {
            match observation {
                ObservationEvent::Updated(_id, node) => {
                    if let Some(new_data) = cod::downcast_rc(node.clone()) {
                        self.data = new_data;
                    }
                    self.tasks.update(state, &self.data.tasks, |state, child_ref| {
                        TaskWidget::new(child_ref).build(state, entity, |builder| builder)
                    });
                },
                _ => {}
            }
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
