use axum::http::StatusCode;
use axum::response::Html;
use tera::Context;
use crate::question_bank::QUESTION_BANK;
use crate::TEMPLATES;

pub async fn get_question(axum::extract::Path(id): axum::extract::Path<String>) -> Result<Html<String>, StatusCode> {
    let questions = &QUESTION_BANK.questions;
    let Some(question) = questions.get(&id) else { return Err(StatusCode::NOT_FOUND) };

    let mut ctx = Context::new();
    ctx.insert("question_id", &id);
    ctx.insert("question_name", &question.name);
    ctx.insert("question_text", &question.text);
    ctx.insert("question_description", &question.description);
    ctx.insert("hints", &question.hints);
    ctx.insert("hints_length", &question.hints.len());

    Ok(Html(TEMPLATES.render("questions/question.html", &ctx).unwrap()))
}