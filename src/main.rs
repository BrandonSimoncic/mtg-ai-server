#![warn(unused_extern_crates)]

mod routes;
mod handlers;
mod formats;

#[tokio::main]
async fn main() {

    let routes = routes::routes();
    println!("Server started at 0.0.0.0 8080");
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}