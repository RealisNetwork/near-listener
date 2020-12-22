
use std::sync::{ Arc, Mutex };
use crate::Capacitor;
use actix_web::{ web, App, HttpServer, HttpRequest, HttpResponse };
use std::env;
use qstring::{ QString };

struct AppState {
    capacitor_ins: Arc<Mutex<Capacitor>>,
}

async fn handle_post_add_account(data: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let api_token = env::var("API_TOKEN").expect("API_TOKEN is required to be defined in the .env file");
    let mut capacitor_ins = data.capacitor_ins.lock().unwrap();
    let query_string = QString::from(req.query_string());
    let req_token = match query_string.get("token") {
        Some(token) => token,
        None => return HttpResponse::BadRequest().body("`token` is a required parameter"),
    };

    if req_token != api_token {
        return HttpResponse::Forbidden().body("Api token did not match");
    }

    let req_account_id = match query_string.get("account_id") {
        Some(account_id) => account_id,
        None => return HttpResponse::BadRequest().body("`account_id` is a required parameter"),
    };

    capacitor_ins.add_account_id(req_account_id.to_string()).await;

    return HttpResponse::Ok().body(format!("Account '{}' was added to the database", &req_account_id));
}

pub async fn start_http_server(capacitor_ins: Arc<Mutex<Capacitor>>) {
    let state = web::Data::new(AppState {
        capacitor_ins,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/config/add_account", web::get().to(handle_post_add_account))
    })
    .bind("127.0.0.1:3000").expect("Could not run http server on that port")
    .run()
    .await.expect("Failed to start http server");
}
