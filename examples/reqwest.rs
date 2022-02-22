use async_compat::Compat;
use bevy::{log::LogPlugin, prelude::*, tasks::IoTaskPool};
use crossbeam_channel::{bounded, Receiver};
use reqwest::Error;

#[derive(Component)]
pub struct Output<T>(pub Receiver<T>);

fn startup(pool: Res<IoTaskPool>, mut commands: Commands) {
    let (sender, receiver) = bounded(1);

    // Construct a Future
    let fut = async move {
        let response = reqwest::get("https://jsonplaceholder.typicode.com/todos/1").await?;
        response.text().await
    };
    let fut = async move {
        sender.send(fut.await).unwrap();
    };

    let task = pool.spawn(Compat::new(fut));

    commands.spawn().insert(Output(receiver)).insert(task);
}

type JSONOutput = Output<Result<String, Error>>;

fn print_result(mut query: Query<&mut JSONOutput>) {
    let output = query.single_mut();
    if let Ok(result) = output.0.try_recv() {
        let text = result.as_ref().expect("failed to load");
        info!(message = "found result", %text);
    }
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugin(LogPlugin)
        // .add_plugin(ExecutorPlugin)
        .add_startup_system(startup)
        .add_system(print_result)
        .run();
}
