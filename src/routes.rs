use warp::Filter;
use super::handlers;
use warp::http::Method;

// A function to build our routes
pub fn routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let cors = warp::cors()
    .allow_methods([Method::GET, Method::POST])
    .allow_any_origin()
    .allow_headers(vec![ "Access-Control-Allow-Origin",
    "Origin",
    "Accept",
    "X-Requested-With",
    "Content-Type",
    "Authorization",
    "HTTP-Referer",
    "X-Title",]);
    let options_route = warp::options()
        .map(|| warp::reply::with_header("OK", "Access-Control-Allow-Origin", "*"));
    get_post().or(options_route).with(cors)
}


// A route to handle GET requests for a specific post
fn get_post() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("posts" / String)
        .and(warp::get())
        .and(warp::header::<String>("Authorization"))
        .and(warp::header::<String>("HTTP-Referer"))
        .and(warp::header::<String>("X-Title"))
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(handlers::get_post)
    }