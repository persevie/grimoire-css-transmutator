use actix_web::{web, HttpResponse, Responder, Result};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use shuttle_actix_web::ShuttleActixWeb;
use std::env;

#[derive(Deserialize)]
struct CssInput {
    css: String,
}

#[derive(Serialize)]
struct JsonResponse {
    duration: String,
    json: String,
}

#[derive(Serialize)]
struct VersionsResponse {
    gcsst_version: String,
    gcsst_ui_version: String,
}

async fn get_versions() -> impl Responder {
    let gcsst_version = env::var("GCSST_VERSION").unwrap_or_else(|_| "unknown".to_string());
    let gcsst_ui_version = env::var("GCSST_UI_VERSION").unwrap_or_else(|_| "unknown".to_string());

    HttpResponse::Ok().json(VersionsResponse {
        gcsst_version,
        gcsst_ui_version,
    })
}

async fn render_index() -> Result<actix_files::NamedFile> {
    Ok(actix_files::NamedFile::open("templates/index.html")?)
}

fn error_response<T: std::fmt::Debug>(err: T) -> HttpResponse {
    HttpResponse::BadRequest().json(JsonResponse {
        json: format!("Error: {:?}", err),
        duration: String::from("N/A"),
    })
}

async fn transmute(input: web::Json<CssInput>) -> impl Responder {
    match gcsst_lib::transmute_from_content(&input.css) {
        Ok((duration, json_output)) => HttpResponse::Ok().json(JsonResponse {
            json: json_output,
            duration: duration.to_string(),
        }),
        Err(err) => error_response(err),
    }
}

#[shuttle_runtime::main]
async fn shuttle_main(
) -> ShuttleActixWeb<impl FnOnce(&mut web::ServiceConfig) + Send + Clone + 'static> {
    dotenv().ok();

    let config = move |cfg: &mut web::ServiceConfig| {
        cfg.service(web::resource("/").route(web::get().to(render_index)))
            .service(web::resource("/transmute").route(web::post().to(transmute)))
            .service(web::resource("/versions").route(web::get().to(get_versions)))
            .service(actix_files::Files::new("/static", "./static").index_file("index.html"));
    };

    Ok(config.into())
}
