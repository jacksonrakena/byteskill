use std::collections::HashMap;
use std::{env, fs};
use std::any::Any;
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
use axum::response::Html;
use axum::{Router, ServiceExt};
use axum::http::StatusCode;
use axum::routing::{get,post};
use handlebars::Handlebars;
use lazy_static::lazy_static;
use serde_json::json;
use tera::{Context, Tera};
use crate::evaluation::RunFailure::{CompilationError, Internal, RuntimeError};
use crate::question_bank::get_questions;
use serde::Deserialize;
use crate::evaluation::Evaluator;

pub mod evaluation;
pub mod question_bank;



async fn question(axum::extract::Path(id): axum::extract::Path<i64>) -> Result<Html<String>, StatusCode> {
    let questions = get_questions();
    let Some(question) = questions.get(&id) else { return Err(StatusCode::NOT_FOUND) };

    let mut ctx = Context::new();
    ctx.insert("question_id", &id);
    ctx.insert("question_name", &question.name);
    ctx.insert("question_text", &question.text);

    Ok(Html(TEMPLATES.render("questions/question.html", &ctx).unwrap()))
}

#[derive(Deserialize)]
struct MarkQuestionFormBody{
    answer: String
}
async fn mark_question(axum::extract::Path(id): axum::extract::Path<i64>,
                       axum::extract::Form(answer): axum::extract::Form<MarkQuestionFormBody>) -> Result<Html<String>, StatusCode> {
    println!("Marking {}, answer is {} chars long", id, answer.answer.len());
    let evaluator = Evaluator{};
    let mut ctx = Context::new();
    let result = evaluator.evaluate_code(answer.answer).await;

    match result {
        Ok(outcome) => {
            ctx.insert("result", "success");
        },
        Err(Internal(internal)) => {
            ctx.insert("result", "internal_error");
            ctx.insert("error", &format!("{:#?}",internal));
        },
        Err(CompilationError { output, .. }) => {
            ctx.insert("result", "compile_failure");
            ctx.insert("compilation", &output.iter().map(escape_newlines_and_spaces).collect::<Vec<String>>())
        }
        Err(RuntimeError { output, .. }) => {
            ctx.insert("result", "runtime_failure");
            ctx.insert("runtime", &output.iter().map(escape_newlines_and_spaces).collect::<Vec<String>>());
        }
    }
    Ok(Html(TEMPLATES.render("questions/marked.html", &ctx).unwrap()))
}

fn escape_newlines_and_spaces(input: &String) -> String {
    input.clone()
    //input.replace("\n","<br />").replace(" ", "&nbsp;")
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/question/:id", get(question))
        .route("/question/:id", post(mark_question));

    // build templates
    TEMPLATES.check_macro_files().unwrap();
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
}
lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html", ".sql"]);
        tera
    };
}
