use axum::Router;
use axum::routing::{get,post};
use lazy_static::lazy_static;
use log::{info, LevelFilter};
use pretty_env_logger::env_logger::{Builder, Target};
use tera::{Tera};
use crate::evaluation::Evaluator;
use crate::routes::get_question::get_question;
use crate::routes::mark_question::mark_question;

pub mod evaluation;
pub mod question_bank;
pub mod routes;


#[tokio::main]
async fn main() {
    Builder::new().filter_module(stringify!(byteskill), LevelFilter::Info).target(Target::Stdout).init();

    info!("Byteskill server starting");

    Evaluator::global_init_evaluator().await;
    let app = Router::new()
        .route("/question/:id", get(get_question))
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
