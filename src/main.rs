use std::collections::HashMap;
use std::{env, fs};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use bollard::container::{AttachContainerOptions, Config, CreateContainerOptions, LogOutput, RemoveContainerOptions, StartContainerOptions};
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::exec::StartExecResults::Attached;
use bollard::image::CreateImageOptions;
use bollard::models::{ExecInspectResponse, HostConfig};
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use tempfile::{tempdir, tempfile};
use tokio::io::AsyncWriteExt;
use std::io::{self, Write};
use crate::InternalError::{ContainerCreationFailure, ContainerStartFailure, EngineUnavailable, FailedCompilationAttach, ImageUnavailable};
use crate::RunFailure::{CompilationError, Internal};

#[derive(Debug)]
enum InternalError {
    EngineUnavailable,
    ImageUnavailable,
    ContainerCreationFailure,
    ContainerStartFailure,
    FailedCompilationAttach(String),
    FailedRuntimeAttach(String)
}

#[derive(Debug)]
enum RunFailure {
    CompilationError { output: Vec<String>, exit_code: i64 },
    RuntimeError { output: Vec<String>, exit_code: i64 },
    Internal(InternalError)
}

type EvaluationResult = Result<Vec<String>, RunFailure>;


async fn evaluate_code(code: String) -> EvaluationResult {
    let Ok(docker) = Docker::connect_with_socket_defaults() else { return Err(Internal(EngineUnavailable)) };
    println!("Initialised Docker");

    // Start image
    let Ok(img) = docker.create_image(Some(CreateImageOptions {
        from_image: "eclipse-temurin:17.0.3_7-jdk-jammy",
        ..Default::default()
    }), None, None).try_collect::<Vec<_>>().await else { return Err(Internal(ImageUnavailable)) };
    println!("Initialised image");

    let Ok(tempfolder) = tempdir() else { return Err(Internal(ContainerCreationFailure)) };
    println!("temporary folder {}", tempfolder.path().display());

    let Ok(mut file) = File::create(tempfolder.path().join("Exercise.java")) else { return Err(Internal(ContainerCreationFailure)) };
    let Ok(()) = writeln!(file, "{}", code) else { return Err(Internal(ContainerCreationFailure)) };
    println!("Wrote data to {}", tempfolder.path().join("Exercise.java").display());

    let Ok(container) = docker.create_container::<&str, &str>(None, Config {
        image: Some("eclipse-temurin:17.0.3_7-jdk-jammy"),
        entrypoint: Some(vec!["tail", "-F", "/dev/null"]),
        host_config: Some(HostConfig {
            binds: Some(vec![format!("{}:/workspace", tempfolder.path().to_string_lossy())]),
            ..HostConfig::default()
        }),
        ..Default::default()
    }).await else { return Err(Internal(ContainerCreationFailure)) };

    println!("Created container {}", container.id);
    docker.start_container(&container.id, None::<StartContainerOptions<String>>).await.unwrap();
    println!("Started container {}", container.id);


    let Ok(compilation_exec) = docker.create_exec(&container.id, CreateExecOptions {
        attach_stderr: Some(true),
        attach_stdout: Some(true),
        cmd: Some(vec!["javac", "-Xlint", "Exercise.java"]),
        working_dir: Some("/workspace"),
        ..Default::default()
    }).await else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };

    let Ok(exec_result) = docker.start_exec(&compilation_exec.id, None).await else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };

    // theoretically unreachable because we never start it in detached mode
    let Attached { mut output, ..} = exec_result else { unreachable!() };

    // Consume compilation results
    let mut o = vec![];
    while let Some(Ok(msg)) = output.next().await {
        o.push(msg.to_string());
    }

    let Ok(ExecInspectResponse { exit_code, .. }) = docker.inspect_exec(&compilation_exec.id).await else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };
    let Some(code) = exit_code else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };

    println!("Finished compilation with code {}.", code);

    if code != 0 {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(CompilationError {exit_code: code, output: o })
    }

    let Ok(runtime_exec) = docker.create_exec(&container.id, CreateExecOptions {
        attach_stderr: Some(true),
        attach_stdout: Some(true),
        cmd: Some(vec!["java", "-ea", "Exercise"]),
        working_dir: Some("/workspace"),
        ..Default::default()
    }).await else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };

    let Ok(exec_result) = docker.start_exec(&runtime_exec.id, None).await else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };

    // theoretically unreachable because we never start it in detached mode
    let Attached { mut output, ..} = exec_result else { unreachable!() };

    // Consume runtime results
    let mut o = vec![];
    while let Some(Ok(msg)) = output.next().await {
        o.push(msg.to_string());
    }

    let Ok(ExecInspectResponse { exit_code, .. }) = docker.inspect_exec(&runtime_exec.id).await else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };
    let Some(code) = exit_code else {
        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
        return Err(Internal(ContainerStartFailure))
    };

    docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
    println!("Removed container.");

    Ok(o)
}

#[tokio::main]
async fn main() {
    match evaluate_code("\
//7 there is record implementing interface, add interface,
//methods is satisfied by field

interface Bird{
    double speed();
    default double flyingSpeed() {
        return this.speed()*10;
    }
}
record Eagle(double speed) implements Bird{}
public class Exercise{
  public static void main(String[] arg){
    Eagle dodo = new Eagle(3);
    Bird a1 = dodo;
    Bird a2 = new Eagle(6);
    assert a1.speed()==3;
    assert a1.flyingSpeed()==30;
    assert a2.speed()==6;
    assert a2.flyingSpeed()==60;
  }
}
    ".to_string()).await {
        Ok(output) => {
            println!("Success: {:?}", output)
        }
        Err(Internal(int)) => {
            println!("Internal error: {:?}", int)
        }
        Err(err) => {
            println!("Error: {:?}", err);
        }
    }
}
