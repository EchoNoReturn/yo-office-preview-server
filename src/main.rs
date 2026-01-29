use axum::{Router, response::Html, routing::get, serve};
use dotenv::dotenv;

mod cache;
mod convert;
mod preview;
mod url_helper;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let app = Router::new()
        .route("/", get(root))
        .route("/preview", get(preview::preview));

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    print!("All cache files will be stored in tmp dir: {}\n", crate::convert::get_output_dir());
    println!("listening on http://{}", listener.local_addr().unwrap());
    let _ = serve(listener, app).await;
}

async fn root() -> Html<String> {
    let html_content = std::fs::read_to_string("static/index.html").unwrap();
    Html(html_content)
}
