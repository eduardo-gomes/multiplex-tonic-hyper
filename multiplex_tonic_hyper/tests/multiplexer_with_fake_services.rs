use tower::ServiceExt;

mod common;
use common::svc;
use multiplex_tonic_hyper::Multiplexer;

#[tokio::test]
async fn multiplexer_propagate_inner_error() {
	let ready = svc::ReadyService {};
	let error = svc::ErrorService {};

	assert!(Multiplexer::new(ready, error).ready().await.is_err());
	assert!(Multiplexer::new(error, ready).ready().await.is_err());
}
