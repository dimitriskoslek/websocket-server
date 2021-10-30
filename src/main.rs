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
	let (tx, _rx) = broadcast::channel(10);

	// Keep accepting sockets
	loop {
		// Accept socket
		let (mut socket, addr) = listener.accept().await.unwrap();

		// In this scope we clone rx and tx because cannot move them into the spawned tasks
		let tx = tx.clone();
		let mut rx = tx.subscribe();

		/*
		** We are about to spawn a task. A bit of wisdom then!
		** When we have a finite amount of things that need to operate
		** in a shared state, we prefer select, as we will see later.
		** In a case like this where we can expect pretty much any amount
		** of incoming connections, we will spawn tasks.
		*/

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
				** In this case we indeed have finite things plus they are dependent
				** on the same source in the way that we splitted the socket to get
				** sender and writer.
				**
				** Note that these are Futures and thus have been already awaited.
				**
				** Also, not that select blocks when a result is selected so the
				** two can't mess with eachother.
				*/
				tokio::select! {
					result = reader.read_line(&mut line) => {
						// If the user disconnected break
						if result.unwrap() == 0 {
							break;
						}

						
						tx.send((line.clone(), addr)).unwrap();
						line.clear();
					}
					result = rx.recv() => {
						let (msg, other_addr) = result.unwrap();

						if addr != other_addr {
							writer.write_all(msg.as_bytes()).await.unwrap();
						}
					}
				}
			}
		});
	}
}
