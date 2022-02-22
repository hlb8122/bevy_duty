use async_compat::Compat;
use bevy::{app::AppExit, log::LogPlugin, prelude::*};
use bevy_duty::{task, ExecutorPlugin, Output};
use reqwest::Error;

fn startup(mut commands: Commands) {
    // Construct a Future
    let fut = async {
        let response = reqwest::get("https://jsonplaceholder.typicode.com/todos/1").await?;
        response.text().await
    };

    // Turn it into a Bundle
    let task = task(Compat::new(fut));

    // Insert the Bundle
    commands.spawn().insert_bundle(task);
}

type JSONOutput = Output<Result<String, Error>>;

fn print_result(query: Query<&JSONOutput, Added<JSONOutput>>, mut writer: EventWriter<AppExit>) {
    if let Ok(result) = query.get_single() {
        let text = result.0.as_ref().expect("failed to load");
        info!(message = "found result", %text);
        writer.send(AppExit);
    }
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugin(LogPlugin)
        .add_plugin(ExecutorPlugin)
        .add_startup_system(startup)
        .add_system(print_result)
        .run();
}
