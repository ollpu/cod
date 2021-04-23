
use tuix::*;
use tuix_reactive::*;
use cod::Node;

use std::rc::Rc;

static STYLE: &str = include_str!("todo_style.css");

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
    editor: Entity,
}

impl TodoApp {
    pub fn new() -> Self {
        // Add an initial state
        let state = cod::State::new(&TodoState::default());

        Self {
            data: state.root_ref(),
            undo_manager: BasicUndoManager::new(state, 128),
            editor: Entity::null(),
        }
    }
}

impl Widget for TodoApp {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        state.set_focus(entity);
        configure_observer(state, entity, ConfigureObserver::RegisterRoot);
        entity 
            .set_background_color(state, Color::rgb(50,50,50))
            .set_layout_type(state, LayoutType::Row)
            .set_width(state, Stretch(1.));
        
        let container = Column::new().build(state, entity, |builder| builder.set_width(Stretch(1.)));

        Textbox::new("Enter new todo here...")
            .on_submit(move |val| Event::new(TodoEvent::Add(val.to_owned())).direct(entity))
            .build(state, container, |builder| 
                builder
                    .set_height(Pixels(30.0))
                    .set_child_space(Stretch(1.0))
                    .set_child_left(Pixels(5.0))
            );
        
        let task_list = TaskList::new(self.data.clone()).build(state, container, |builder| builder);
        self.editor = TaskEditor::default().build(state, entity, |builder|
            builder
                .set_width(Stretch(1.))
                .set_height(Stretch(1.))
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
                            state.insert_event(Event::new(TodoEvent::Add("Test".to_owned())).direct(entity));
                        },
                        Code::KeyD if state.modifiers.ctrl => {
                            state.insert_event(Event::new(TodoEvent::Debug).direct(entity));
                        },
                        Code::KeyZ if state.modifiers.ctrl && state.modifiers.shift => {
                            state.insert_event(Event::new(TodoEvent::Redo).direct(entity));
                        },
                        Code::KeyZ if state.modifiers.ctrl => {
                            state.insert_event(Event::new(TodoEvent::Undo).direct(entity));
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
                    
                    state.insert_event(Event::new(TodoEvent::StartUndoState).direct(entity));
                    // Mutate the app state to add the new task
                    mutate(state, entity, &self.data, move |data| {
                        data.tasks.push(cod::Child::with_parent(&*data, Task {
                            header: Default::default(),
                            description: task.clone(),
                            completed: false,
                        }));
                    });

                    //state.set_focus(entity);
                }
                TodoEvent::Debug => {
                    println!("{:?}", self.data);
                }
                TodoEvent::Remove(id) => {
                    state.insert_event(Event::new(TodoEvent::StartUndoState).direct(entity));
                    mutate(state, entity, &self.data, move |data| {
                        data.tasks.retain(|t| t.get_id() != id);
                    });
                }
                TodoEvent::Edit(id) => {
                    state.insert_event(Event::new(TodoEvent::Edit(id)).direct(self.editor));
                }
                TodoEvent::StartUndoState => {
                    self.undo_manager.start_undo_state();
                }
                TodoEvent::Undo => {
                    self.undo_manager.undo(state);
                }
                TodoEvent::Redo => {
                    self.undo_manager.redo(state);
                }
            }
        }

        if let Some(update) = downcast_update(event) {
            match update {
                UpdateEvent::Update(node, _animate) => {
                    self.data = node;
                }
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
            .set_background_color(state, Color::rgb(200,200,200))
            .set_child_top(state, Pixels(2.))
            .set_child_between(state, Pixels(2.))
    }

    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(update) = downcast_update(event) {
            match update {
                UpdateEvent::Update(node, animate) => {
                    self.data = node;
                    self.tasks.update(state, &self.data.tasks, animate, |state, container, child_ref| {
                        TaskWidget::new(child_ref).build(state, container, |builder| builder)
                    });
                }
                _ => {}
            }
        }
    }
}



#[derive(Default)]
struct TaskEditor {
    title: Entity,
    textbox: Entity,
    edit_latch: bool,
    task: Option<Rc<Task>>
}

#[derive(Clone, PartialEq, Debug)]
enum EditorEvent {
    ChangeDescription(String),
    Delete,
}

impl Widget for TaskEditor {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self:: Ret {
        entity
            .set_background_color(state, Color::rgb(70, 70, 70))
            .set_child_space(state, Pixels(8.))
            .set_child_between(state, Pixels(8.));
        self.title = Label::new("-").build(state, entity, |b| b.set_font_size(24.).set_height(Pixels(24.)));
        Row::new().build(state, entity, |b| b.set_background_color(Color::white()).set_height(Pixels(1.)));
        self.textbox = Textbox::new("")
            .on_change(move |text| Event::new(EditorEvent::ChangeDescription(text.to_owned())).direct(entity))
            .build(state, entity, |b| b
                   //.set_background_color(Color::rgb(100, 100, 100))
                   .set_height(Pixels(30.))
                   .set_width(Stretch(1.))
                   .set_border_width(Pixels(2.))
                   .set_child_space(Stretch(1.0))
                   .set_child_left(Pixels(5.))
                   .set_background_color(Color::rgb(50, 50, 50))
                   .set_border_color(Color::rgb(100, 100, 100))
                  );
        Button::with_label("Delete")
            .on_release(Event::new(EditorEvent::Delete).direct(entity))
            .build(state, entity, |b| b
                   .set_height(Pixels(30.))
                   .set_width(Pixels(80.))
                   .set_right(Stretch(1.))
                   .set_background_color(Color::rgb(90, 90, 100))
                   .set_child_space(Stretch(1.0))
                  );
        entity
    }
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(todo_event) = event.message.downcast() {
            match todo_event {
                TodoEvent::Edit(id) => {
                    configure_observer(state, entity, ConfigureObserver::Replace(*id));
                }
                _ => {}
            }
        }
        if let Some(update) = downcast_update::<Task>(event) {
            match update {
                UpdateEvent::Update(node, _animate) => {
                    self.title.set_text(state, &format!("Task #{}", node.header.id()));
                    state.insert_event(Event::new(TextboxEvent::SetValue(node.description.clone())).target(self.textbox));
                    self.task = Some(node);
                }
                UpdateEvent::Remove(_id, _animate) => {
                    self.task = None;
                    self.title.set_text(state, "-");
                    state.insert_event(Event::new(TextboxEvent::SetValue("".to_owned())).target(self.textbox));
                }
            }
        }
        if let Some(edit) = event.message.downcast() {
            use EditorEvent::*;
            match edit {
                ChangeDescription(text) => {
                    if let Some(task) = &self.task {
                        if self.edit_latch {
                            self.edit_latch = false;
                            state.insert_event(Event::new(TodoEvent::StartUndoState).target(entity).propagate(Propagation::Up));
                        }
                        let text = text.clone();
                        mutate(state, entity, task, move |data| {
                            data.description = text.clone(); // FIXME
                        });
                    }
                }
                Delete => {
                    if let Some(task) = &self.task {
                        state.insert_event(Event::new(TodoEvent::Remove(task.header.id())).target(entity).propagate(Propagation::Up));
                    }
                }
            }
        }
        if let Some(window_event) = event.message.downcast() {
            match window_event {
                WindowEvent::FocusIn => {
                    self.edit_latch = true;
                }
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
        initial_update(self, state, entity, Rc::clone(&self.task));
        entity
            .set_height(state, Pixels(50.0))
            .set_background_color(state, Color::rgb(80,80,80))
            .set_child_left(state, Pixels(5.0))
    }
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(update) = downcast_update(event) {
            match update {
                UpdateEvent::Update(node, _animate) => {
                    self.task = node;
                    entity.set_text(state, &self.task.description);
                }
                UpdateEvent::Remove(_id, _animate) => {
                    state.remove(entity);
                }
            }
        }
        if let Some(window_event) = event.message.downcast() {
            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    state.insert_event(Event::new(TodoEvent::Edit(self.task.header().id())).propagate(Propagation::All));
                },
                _ => {}
            }
        }
        if let Some(todo_event) = event.message.downcast() {
            match todo_event {
                TodoEvent::Edit(id) => {
                    if self.task.header.id() == *id {
                        entity.set_background_color(state, Color::rgb(80, 85, 128));
                    } else {
                        entity.set_background_color(state, Color::rgb(80, 80, 80));
                    }
                }
                _ => {}
            }
        }
        if let Some(request) = event.message.downcast() {
            match request {
                AnimationRequest::Appear => {
                    let anim = AnimationState::new()
                        .with_duration(std::time::Duration::from_secs_f32(0.2))
                        .with_keyframe((0.0, Pixels(0.0)))
                        .with_keyframe((1.0, Pixels(50.0)));
                    let anim = state.style.height.insert_animation(anim);
                    state.style.height.play_animation(entity, anim);
                },
                _ => {}
            }
        }
    }
}



fn main() {
    let app = Application::new(WindowDescription::new().with_title("Tuix Todos"), |state, window| {
        state.add_theme(STYLE);
        TodoApp::new().build(state, window.entity(), |builder| builder);
    });

    app.run();
}
