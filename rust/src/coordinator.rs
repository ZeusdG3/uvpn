use crate::messages::{Task, ResultMsg};
use tokio::net::TcpListener;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

pub async fn run_coordinator() -> tokio::io::Result<()> {
	let listener = TcpListener::bind("0.0.0.0:8080").await?;
	println!("Coordinador escuchando en todos lados");

	loop {
		let (mut socket, _) = listener.accept().await?;
		println!("Worker conectado");

		//Aqui envia la tarea dummy
		let task = Task { id: 1, payload: "dummy".to_string() };
		let serialized = serde_json::to_string(&task).unwrap();
		socket.write_all(serialized.as_bytes()).await?;

		//Aqui recibe el resultado
		let mut buf = vec![0; 1024];
		let n = socket.read(&mut buf).await?;
		let response: ResultMsg = serde_json::from_slice(&buf[..n]).unwrap();
		println!("Resultado recibido: {:?}", response);
		
	}
}
