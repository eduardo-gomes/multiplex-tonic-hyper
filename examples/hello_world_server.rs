use std::convert::Infallible;

use hello_world_tonic::hello_world::greeter_server::GreeterServer;
use hello_world_tonic::server::MyGreeter;
use hyper::server::conn::AddrStream;
use hyper::service::make_service_fn;
use hyper::{service::service_fn, Body, Response};
use tower::make::Shared;

use multiplex_tonic_hyper::MakeMultiplexer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let addr = "[::]:9999".parse()?;
	let greeter = MyGreeter::default();

	let greeter_service = Shared::new(GreeterServer::new(greeter));
	let web_service = make_service_fn(move |conn: &AddrStream| {
		let addr = conn.remote_addr();
		let service = service_fn(move |req| {
			let info = format!(
				"Got web request from: {}, method: {}, path: {}, version: {:?}",
				addr,
				req.method(),
				req.uri(),
				req.version()
			);
			println!("{info}");
			let response = format!("Hello World\n\n\n\n{req:#?}");
			let res: Result<Response<Body>, Infallible> = Ok(Response::new(Body::from(response)));
			async { res }
		});
		async move { Ok::<_, Infallible>(service) }
	});

	let make_multiplexer = MakeMultiplexer::new(greeter_service, web_service);

	let server = hyper::Server::bind(&addr).serve(make_multiplexer);
	println!(
		"Try the web service at http://[::1]:{}",
		server.local_addr().port()
	);
	server.await?;
	Ok(())
}
