// use axum::body::Body;
// use axum::extract::Request;
// use axum::response::{IntoResponse, Response};
// use axum::Router;
// use axum::routing::get;
// use http::{Method, StatusCode};
// use hyper::body::Incoming;
// use hyper::upgrade::Upgraded;
// use hyper_util::rt::TokioIo;
// use tokio::net::TcpStream;
// use tokio::sync::OnceCell;
// use tower::{Service, ServiceExt};
// use hyper::service;
//
//
//
// pub async fn init() {
//     HYPER_SERVICE_ONCE.get_or_init(|| async move{
//         let router_svc = Router::new().route("/", get(|| async { "Hello, World!" }));
//
//         let tower_service = tower::service_fn(move |req: Request<_>| {
//             let router_svc = router_svc.clone();
//             let req = req.map(Body::new);
//             async move {
//                 if req.method() == Method::CONNECT {
//                     proxy(req).await
//                 } else {
//                     router_svc.oneshot(req).await.map_err(|err| match err {})
//                 }
//             }
//         });
//
//         let hyper_service = hyper::service::service_fn(move |request: Request<Incoming>| {
//             tower_service.clone().call(request)
//         });
//         hyper_service
//     })
// }
//
//
// pub async fn handle(stream: TcpStream) {
//     let io = TokioIo::new(stream);
//     let hyper_service = hyper_service.clone();
//     tokio::task::spawn(async move {
//         if let Err(err) = http1::Builder::new()
//             .preserve_header_case(true)
//             .title_case_headers(true)
//             .serve_connection(io, hyper_service)
//             .with_upgrades()
//             .await
//         {
//             println!("Failed to serve connection: {:?}", err);
//         }
//     });
// }
//
// pub async fn proxy(req: Request) -> Result<Response, hyper::Error> {
//     tracing::trace!(?req);
//     if let Some(host_addr) = req.uri().authority().map(|auth| auth.to_string()) {
//         tokio::task::spawn(async move {
//             match hyper::upgrade::on(req).await {
//                 Ok(upgraded) => {
//                     if let Err(e) = tunnel(upgraded, host_addr).await {
//                         tracing::warn!("server io error: {}", e);
//                     };
//                 }
//                 Err(e) => tracing::warn!("upgrade error: {}", e),
//             }
//         });
//
//         Ok(Response::new(Body::empty()))
//     } else {
//         tracing::warn!("CONNECT host is not socket addr: {:?}", req.uri());
//         Ok((
//             StatusCode::BAD_REQUEST,
//             "CONNECT must be to a socket address",
//         )
//             .into_response())
//     }
// }
//
// async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
//     let mut server = TcpStream::connect(addr).await?;
//     let mut upgraded = TokioIo::new(upgraded);
//
//     let (from_client, from_server) =
//         tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;
//
//     tracing::debug!(
//         "client wrote {} bytes and received {} bytes",
//         from_client,
//         from_server
//     );
//
//     Ok(())
// }