# Multiplex tonic hyper

- Note: This is the first crate I made. Just publishing to try [crates.io](https://crates.io/)

This implements a service that routes requests based on `Content-Type`. If starts with `application/grpc` it sends to
the inner gRPC service. If it doesn't, then it sends to the other service.

### Examples

Try the examples:

- A [gRPC/web server](examples/hello_world_server.rs).

Open the server, them try the web service at [http://[::1]:9999](http://[::1]:9999).

```sh
cargo run --example hello_world_server
```

- [gRPC client](examples/hello_world_client.rs).

```sh
cargo run --example hello_world_client
```