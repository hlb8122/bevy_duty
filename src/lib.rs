use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bevy::{ecs::system::EntityCommands, prelude::*};
use crossbeam_channel::{unbounded, Receiver, Sender};
use waker_fn::waker_fn;

#[derive(Component)]
pub struct Awake;

#[derive(Component)]
pub struct Output<T>(pub T);

type BoxEntityCommands = Box<dyn FnOnce(&mut EntityCommands) + 'static>;

/// Represents an async task. Spawning this
#[derive(Component)]
struct Task(Pin<Box<dyn Future<Output = BoxEntityCommands> + Send + Sync + 'static>>);

/// Constructs a [`Bundle`] which, when inserted into an entity, will cause an [`Output`] to be
/// added when the [`Future`] resolves.
pub fn task<F>(fut: F) -> impl Bundle
where
    F: Future + Send + Sync + 'static,
    F::Output: Send + Sync + 'static,
{
    let fut = async move {
        let output = fut.await;

        let f = Box::new(move |entity_commands: &mut EntityCommands| {
            entity_commands.insert(Output(output));
        }) as Box<dyn FnOnce(&mut EntityCommands)>;
        f
    };
    (Task(Box::pin(fut)), Awake)
}

struct Executor {
    sender: Sender<Entity>,
    receiver: Receiver<Entity>,
}

fn wake_system(res: Res<Executor>, mut commands: Commands) {
    for id in res.receiver.try_iter() {
        trace!(message = "waking entity", ?id);
        commands.entity(id).insert(Awake);
    }
}

fn poll_system(
    res: ResMut<Executor>,
    mut query: Query<(Entity, &mut Task), With<Awake>>,
    mut commands: Commands,
) {
    for (id, mut task) in query.iter_mut() {
        let sender = res.sender.clone();
        let waker = waker_fn(move || {
            sender.send(id).expect("executor dropped");
        });
        let mut cx = Context::from_waker(&waker);
        let result = task.0.as_mut().poll(&mut cx);

        let mut entity_commands = commands.entity(id);
        entity_commands.remove::<Awake>();
        match result {
            Poll::Ready(output) => output(&mut entity_commands),
            Poll::Pending => (),
        }
    }
}

pub struct ExecutorPlugin;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum ExecutorPhase {
    Wake,
    Poll,
}

impl SystemLabel for ExecutorPhase {
    fn dyn_clone(&self) -> Box<dyn SystemLabel> {
        Box::new(self.clone())
    }
}

impl Plugin for ExecutorPlugin {
    fn build(&self, app: &mut App) {
        let poll_set = SystemSet::new()
            .label(ExecutorPhase::Poll)
            .before(ExecutorPhase::Wake)
            .with_system(poll_system);
        let wake_set = SystemSet::new()
            .label(ExecutorPhase::Wake)
            .with_system(wake_system);
        let (sender, receiver) = unbounded();
        let executor = Executor { sender, receiver };
        app.insert_resource(executor)
            .add_system_set(wake_set)
            .add_system_set(poll_set);
    }
}
