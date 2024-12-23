use actix_web::{web, HttpResponse, Responder, Result};
use gcsst_lib::transmute_from_content;
use serde::{Deserialize, Serialize};
use shuttle_actix_web::ShuttleActixWeb;

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

#[derive(Clone)]
struct AppState {
    gcsst_version: String,
    gcsst_ui_version: String,
}

async fn get_versions(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(VersionsResponse {
        gcsst_version: data.gcsst_version.clone(),
        gcsst_ui_version: data.gcsst_ui_version.clone(),
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
    match transmute_from_content(&input.css) {
        Ok((duration, json_output)) => HttpResponse::Ok().json(JsonResponse {
            json: json_output,
            duration: duration.to_string(),
        }),
        Err(err) => error_response(err),
    }
}

#[shuttle_runtime::main]
async fn shuttle_main(
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> ShuttleActixWeb<impl FnOnce(&mut web::ServiceConfig) + Send + Clone + 'static> {
    let gcsst_version = secrets
        .get("GCSST_VERSION")
        .unwrap_or("unknown".to_string());
    let gcsst_ui_version = secrets
        .get("GCSST_UI_VERSION")
        .unwrap_or("unknown".to_string());

    let app_state = web::Data::new(AppState {
        gcsst_version,
        gcsst_ui_version,
    });

    let config = move |cfg: &mut web::ServiceConfig| {
        cfg.app_data(app_state.clone())
            .service(web::resource("/").route(web::get().to(render_index)))
            .service(web::resource("/transmute").route(web::post().to(transmute)))
            .service(web::resource("/versions").route(web::get().to(get_versions)))
            .service(actix_files::Files::new("/static", "./static").index_file("index.html"));
    };

    Ok(config.into())
}
