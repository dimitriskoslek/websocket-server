use tokio::{
	io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
	net::TcpListener,
	sync::broadcast,
};

#[tokio::main]
async fn main() {
	// Create a listener and accept socket connections
	let listener = TcpListener::bind("localhost:8080").await.unwrap();

	// Create a sender/receiver broadcast channel (capacity)
	let (tx, _rx) = broadcast::channel::<String>(10);

	// Keep accepting sockets
	loop {
		// Accept socket
		let (mut socket, addr) = listener.accept().await.unwrap();

		// In this scope we clone rx and tx because cannot move them into the spawned tasks
		let tx = tx.clone();
		let mut rx = tx.subscribe();

		// Spawn a task for this socket
		tokio::spawn(async move {

			// Split the write and read part to handle them independently
			let (reader, mut writer) = socket.split();

			let mut reader = BufReader::new(reader);
			let mut line = String::new();

			// Keep accepting input
			loop {
				/*
				** Instead of declaring multiple awaits (and having to split
				** them in seperate tasks at that so that they don't block and
				** interfere with eachother's read/write) we use tokio's select
				** macro. Basically we an outcome depending on what happens.
				**
				** Note that these are Futures and thus have been already awaited.
				*/
				tokio::select! {
					result = reader.read_line(&mut line) => {
						// If the user disconnected break
						if result.unwrap() == 0 {
							break;
						}
						tx.send(line.clone()).unwrap();
						line.clear();
					}
					result = rx.recv() => {
						let msg = result.unwrap();
						writer.write_all(msg.as_bytes()).await.unwrap();						
					}
				}
			}
		});
	}
}
