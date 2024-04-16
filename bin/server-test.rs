use nbd::server::Server;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    println!("Hello test");
    let listener = TcpListener::bind("0.0.0.0:10809").await.unwrap();
    loop {
        let (x, _) = listener.accept().await.unwrap();
        let mut server = Server::new(x);
        server.handshake().await.unwrap();
        server.handle_transmission().await.unwrap();
    }
}
