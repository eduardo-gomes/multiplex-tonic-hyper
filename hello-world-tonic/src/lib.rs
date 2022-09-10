pub mod hello_world {
	tonic::include_proto!("helloworld");
}

pub mod server {
	use super::hello_world;
	use hello_world::greeter_server::Greeter;
	use hello_world::{HelloReply, HelloRequest};
	use tonic::{Request, Response, Status};

	#[derive(Debug, Default)]
	pub struct MyGreeter {}

	#[tonic::async_trait]
	impl Greeter for MyGreeter {
		async fn say_hello(
			&self,
			request: Request<HelloRequest>,
		) -> Result<Response<HelloReply>, Status> {
			let name = &request.get_ref().name;
			println!(
				"Received request with message: '{name}', and headers: {:?}",
				request.metadata().clone().into_headers()
			);

			let reply = hello_world::HelloReply {
				message: format!("Hello {}!", name).into(),
			};

			Ok(Response::new(reply))
		}
	}
}
