use axum::http::StatusCode;
use axum::response::Html;
use tera::Context;
use crate::evaluation::Evaluator;
use crate::evaluation::RunFailure::{CompilationError, Internal, RuntimeError};
use crate::TEMPLATES;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct MarkQuestionFormBody{
    answer: String
}

pub async fn mark_question(axum::extract::Path(id): axum::extract::Path<String>,
                       axum::extract::Form(answer): axum::extract::Form<MarkQuestionFormBody>) -> Result<Html<String>, StatusCode> {
    let evaluator = Evaluator{};
    let mut ctx = Context::new();
    let result = evaluator.evaluate_code(answer.answer).await;

    match result {
        Ok(_) => {
            ctx.insert("result", "success");
        },
        Err(Internal(internal)) => {
            ctx.insert("result", "internal_error");
            ctx.insert("error", &format!("{:#?}",internal));
        },
        Err(CompilationError { output, .. }) => {
            ctx.insert("result", "compile_failure");
            ctx.insert("compilation", &output)
        }
        Err(RuntimeError { output, .. }) => {
            ctx.insert("result", "runtime_failure");
            ctx.insert("runtime", &output);
        }
    }
    Ok(Html(TEMPLATES.render("questions/marked.html", &ctx).unwrap()))
}