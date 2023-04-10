use std::fs::File;
use bollard::container::{Config, RemoveContainerOptions, StartContainerOptions};
use bollard::Docker;
use bollard::exec::{CreateExecOptions};
use bollard::exec::StartExecResults::Attached;
use bollard::image::CreateImageOptions;
use bollard::models::{ExecInspectResponse, HostConfig};
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use tempfile::{tempdir};
use std::io::{Write};
use log::info;

use crate::evaluation::InternalError::{ContainerCreationFailure, ContainerStartFailure, EngineUnavailable};
use crate::evaluation::RunFailure::{CompilationError, Internal, RuntimeError};

pub const DOCKER_IMAGE: &str = "eclipse-temurin:17-jdk-alpine";

#[derive(Debug)]
pub enum InternalError {
    EngineUnavailable,
    ImageUnavailable,
    ContainerCreationFailure,
    ContainerStartFailure,
    FailedCompilationAttach(String),
    FailedRuntimeAttach(String)
}

#[derive(Debug)]
pub enum RunFailure {
    CompilationError { output: Vec<String>, exit_code: i64 },
    RuntimeError { output: Vec<String>, exit_code: i64 },
    Internal(InternalError)
}

type EvaluationResult = Result<Vec<String>, RunFailure>;

pub struct Evaluator {

}

impl Evaluator {
    /// Initialises the global state for all evaluators.
    ///
    /// This should be called at startup to ensure that the Docker base image
    /// is ready for use.
    pub async fn global_init_evaluator() {
        let docker = Docker::connect_with_socket_defaults().unwrap();
        docker.create_image(Some(CreateImageOptions {
            from_image: DOCKER_IMAGE,
            ..Default::default()
        }), None, None).try_collect::<Vec<_>>().await.unwrap();

        info!("Created Docker image {}", DOCKER_IMAGE);
    }

    pub async fn evaluate_code(&mut self, code: String) -> EvaluationResult {
        let Ok(docker) = Docker::connect_with_socket_defaults() else { return Err(Internal(EngineUnavailable)) };

        let Ok(tempfolder) = tempdir() else { return Err(Internal(ContainerCreationFailure)) };

        let Ok(mut file) = File::create(tempfolder.path().join("Exercise.java")) else { return Err(Internal(ContainerCreationFailure)) };
        let Ok(()) = writeln!(file, "{}", code) else { return Err(Internal(ContainerCreationFailure)) };

        let Ok(container) = docker.create_container::<&str, &str>(None, Config {
            image: Some(DOCKER_IMAGE),
            entrypoint: Some(vec!["tail", "-F", "/dev/null"]),
            host_config: Some(HostConfig {
                binds: Some(vec![format!("{}:/workspace", tempfolder.path().to_string_lossy())]),
                ..HostConfig::default()
            }),
            ..Default::default()
        }).await else { return Err(Internal(ContainerCreationFailure)) };

        docker.start_container(&container.id, None::<StartContainerOptions<String>>).await.unwrap();

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
        let Attached { mut output, .. } = exec_result else { unreachable!() };

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

        if code != 0 {
            docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
            return Err(CompilationError { exit_code: code, output: o })
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
        let Attached { mut output, .. } = exec_result else { unreachable!() };

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

        if code != 0 {
            return Err(RuntimeError {exit_code: code, output: o });
        }

        docker.remove_container(&container.id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();

        Ok(o)
    }
}