use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        // Spawn a new async task
        // Move converts any variables captured by reference or mutable reference to variables captured by value
        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    // Match on the number of bytes read
                    // If we read zero bytes than the socket has been closed so we just return
                    Ok(n) if n == 0 => return,

                    // If the socket hasn't been closed then n is set to the number of values that have been read
                    Ok(n) => n,

                    // Check for an error and exit if exists
                    Err(e) => {
                        eprint!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                // Write back to the same socket
                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprint!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
