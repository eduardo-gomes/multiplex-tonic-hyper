use hello_world_tonic::hello_world::greeter_client::GreeterClient;
use hello_world_tonic::hello_world::HelloRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut client = GreeterClient::connect("http://[::1]:9999").await?;

	let request = tonic::Request::new(HelloRequest {
		name: "World".into(),
	});

	let response = client.say_hello(request).await?;

	println!("Got response: {:#?}", response);

	Ok(())
}
