
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
    Add(String),
    Remove(cod::ID),
    Edit(cod::ID),
    Debug,
    StartUndoState,
    Undo,
    Redo,
}

struct TodoApp {
    undo_manager: BasicUndoManager<TodoState>,

    data: Rc<TodoState>,
    //tasks: VecDiffer<Task>,
    editor: Entity,
}

impl TodoApp {
    pub fn new() -> Self {
        // Add an initial state
        let state = cod::State::new(&TodoState::default());

        Self {
            data: state.root_ref(),
            undo_manager: BasicUndoManager::new(state, 128),
            //tasks: Default::default(),
            editor: Entity::null(),
        }
    }
}

impl Widget for TodoApp {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        state.focused = entity;
        configure_observer(state, entity, ConfigureObserver::RegisterRoot);
        entity 
            .set_background_color(state, Color::rgb(50,50,50))
            .set_flex_direction(state, FlexDirection::Row)
            .set_flex_grow(state, 1.0);
        
        let container = VBox::new().build(state, entity, |builder| builder.set_flex_grow(1.0));

        Textbox::new("Enter new todo here...")
        .on_submit(move |val| Event::new(TodoEvent::Add(val.to_owned())).target(entity))
        .build(state, container, |builder| 
            builder
                .set_height(Length::Pixels(30.0))
                .set_padding_left(Length::Pixels(5.0))
        );
        
        let task_list = TaskList::new(self.data.clone()).build(state, container, |builder| builder);
        // let container = VBox::new().build(state, entity, |builder| {
        //     builder.set_flex_grow(1.0)
        // });
        //self.tasks.set_container(container);
        self.editor = TaskEditor::default().build(state, entity, |builder|
            builder
                .set_flex_grow(1.0)
                .set_background_color(Color::rgb(100,100,100))
        );
        entity
    }

    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        self.undo_manager.on_event(state, event);

        // Handle Window Events
        if let Some(window_event) = event.message.downcast() {
            match window_event {
                WindowEvent::KeyDown(code, _) => {
                    match *code {
                        Code::KeyA if state.modifiers.ctrl => {
                            state.insert_event(Event::new(TodoEvent::Add("Test".to_owned())).target(entity));
                        },
                        Code::KeyD if state.modifiers.ctrl => {
                            state.insert_event(Event::new(TodoEvent::Debug).target(entity));
                        },
                        Code::KeyZ if state.modifiers.ctrl && state.modifiers.shift => {
                            state.insert_event(Event::new(TodoEvent::Redo).target(entity));
                        },
                        Code::KeyZ if state.modifiers.ctrl => {
                            state.insert_event(Event::new(TodoEvent::Undo).target(entity));
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Handle Custom Todo Events
        if let Some(todo_event) = event.message.downcast::<TodoEvent>() {
            match todo_event.clone() {
                TodoEvent::Add(task) => {
                    println!("Add a Task");
                    
                    state.insert_event(Event::new(TodoEvent::StartUndoState).target(entity));
                    // Mutate the app state to add the new task
                    mutate(state, entity, &self.data, move |data| {
                        data.tasks.push(cod::Child::with_parent(&*data, Task {
                            header: Default::default(),
                            description: task.clone(),
                            completed: false,
                        }));
                    });
                    event.consume();

                    state.focused = entity;
                }
                TodoEvent::Debug => {
                    println!("{:?}", self.data);
                    event.consume();
                }
                TodoEvent::Remove(id) => {
                    state.insert_event(Event::new(TodoEvent::StartUndoState).target(entity));
                    mutate(state, entity, &self.data, move |data| {
                        data.tasks.retain(|t| t.get_id() != id);
                    });
                    event.consume();
                }
                TodoEvent::Edit(id) => {
                    state.insert_event(Event::new(TodoEvent::Edit(id)).target(self.editor).propagate(Propagation::Direct));
                    event.consume();
                }
                TodoEvent::StartUndoState => {
                    self.undo_manager.start_undo_state();
                    event.consume();
                }
                TodoEvent::Undo => {
                    self.undo_manager.undo(state);
                    event.consume();
                }
                TodoEvent::Redo => {
                    self.undo_manager.redo(state);
                    event.consume();
                }
                _=> {}
            }
        }

        if let Some(observation) = event.message.downcast() {
            match observation {
                ObservationEvent::Updated(_id, node, animate) => {
                    if let Some(new_data) = cod::downcast_rc(node.clone()) {
                        self.data = new_data;
                    }
                },
                _ => {}
            }
        }
    }
}

#[derive(Default)]
struct TaskList {
    data: Rc<TodoState>,
    tasks: VecDiffer<Task>,
}

impl TaskList {
    pub fn new(data: Rc<TodoState>) -> Self {
        Self {
            data: data.clone(),
            tasks: Default::default(),
        }
    }
}

impl Widget for TaskList {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        configure_observer(state, entity, ConfigureObserver::RegisterRoot);

        self.tasks.set_container(entity);

        state.focused = entity;
        entity
            .set_flex_grow(state, 1.0)
            .set_background_color(state, Color::rgb(200,200,200))
    }

    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(observation) = event.message.downcast() {
            match observation {
                ObservationEvent::Updated(_id, node, animate) => {
                    if let Some(new_data) = cod::downcast_rc(node.clone()) {
                        self.data = new_data;
                    }
                    self.tasks.update(state, &self.data.tasks, *animate, |state, container, child_ref| {
                        TaskWidget::new(child_ref).build(state, container, |builder| builder)
                    });
                },
                _ => {}
            }
        }
    }
}



#[derive(Default)]
struct TaskEditor {
    task: Option<Rc<Task>>
}

impl Widget for TaskEditor {
    type Ret = Entity;
    fn on_build(&mut self, _state: &mut State, entity: Entity) -> Self:: Ret {
        entity
    }
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(todo_event) = event.message.downcast() {
            match todo_event {
                TodoEvent::Edit(id) => {
                    configure_observer(state, entity, ConfigureObserver::Register(*id));
                }
                _ => {}
            }
        }
        if let Some(observation) = event.message.downcast() {
            match observation {
                ObservationEvent::Updated(id, node, _animate) => {
                    if let Some(new_data) = cod::downcast_rc(Rc::clone(&node)) {
                        self.task = Some(new_data);
                        println!("moi2");
                    }
                    entity.set_text(state, &format!("{}", id));
                }
                ObservationEvent::Removed(_id) => {
                    self.task = None;
                    entity.set_text(state, "-");
                }
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
        state.insert_event(Event::new(UpdateEvent::Update(Rc::clone(&self.task), false)).target(entity));
        entity
            .set_flex_basis(state, Length::Pixels(50.0))
            .set_background_color(state, Color::rgb(80,80,80))
            .set_margin(state, Length::Pixels(5.0))
            .set_padding_left(state, Length::Pixels(5.0))
    }
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(update) = event.message.downcast() {
            match update {
                UpdateEvent::Update(new_node, _animate) => {
                    self.task = Rc::clone(new_node);
                    entity.set_text(state, &self.task.description);
                },
                UpdateEvent::Remove => {
                    state.remove(entity);
                }
            }
        }
        if let Some(window_event) = event.message.downcast() {
            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    state.insert_event(Event::new(TodoEvent::Edit(self.task.header().id())).propagate(Propagation::Up).target(entity));
                },
                _ => {}
            }
        }
        if let Some(request) = event.message.downcast() {
            match request {
                AnimationRequest::Appear => {
                    let anim = AnimationState::new()
                        .with_duration(std::time::Duration::from_secs_f32(0.2))
                        .with_keyframe((0.0, Length::Pixels(0.0)))
                        .with_keyframe((1.0, Length::Pixels(50.0)));
                    let anim = state.style.flex_basis.insert_animation(anim);
                    state.style.flex_basis.play_animation(entity, anim);
                },
                _ => {}
            }
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
