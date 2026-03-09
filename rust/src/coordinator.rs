use crate::messages::{Task, ResultMsg};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{timeout, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use image::{ImageBuffer, Rgb};
use std::sync::atomic::{AtomicBool, Ordering};
use std::process;

// Parámetros globales (tomados del main original)
const WIDTH: u32 = 7680;
const HEIGHT: u32 = 4320;
const MAX_ITER: u32 = 500;
const SUPERSAMPLING: u32 = 1;

// Región a renderizar (misma que en el original)
const CENTER_RE: f64 = -0.4049987;
const CENTER_IM: f64 = -0.5903320;
const ZOOM: f64 = 10.0;
const FULL_WIDTH: f64 = 3.0;

pub async fn run_coordinator() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Coordinador escuchando en 0.0.0.0:8080 (Docker: coordinator:8080)");

    // Calcular los límites de la región
    let width = FULL_WIDTH / ZOOM;
    let height = width * (HEIGHT as f64 / WIDTH as f64);
    let x_min = CENTER_RE - width / 2.0;
    let x_max = CENTER_RE + width / 2.0;
    let y_min = CENTER_IM - height / 2.0;
    let y_max = CENTER_IM + height / 2.0;

    println!("Región a renderizar:");
    println!("  x: [{:.6}, {:.6}]", x_min, x_max);
    println!("  y: [{:.6}, {:.6}]", y_min, y_max);
    println!("  Iteraciones: {}", MAX_ITER);
    println!("  Supersampling: {}x{}", SUPERSAMPLING, SUPERSAMPLING);

    // Crear tareas: dividir la imagen en bandas horizontales
    let num_tasks = 16;
    let band_height = HEIGHT / num_tasks;
    let mut tasks = Vec::new();
    for i in 0..num_tasks {
        let y_start = i * band_height;
        let y_end = if i == num_tasks - 1 { HEIGHT } else { (i + 1) * band_height };
        tasks.push(Task {
            id: i,
            x_min,
            x_max,
            y_min,
            y_max,
            width: WIDTH,
            global_height: HEIGHT,
            y_start,
            y_end,
            max_iter: MAX_ITER,
            supersampling: SUPERSAMPLING,
        });
    }

    let tasks_arc = Arc::new(Mutex::new(tasks));
    let results_arc = Arc::new(Mutex::new(HashMap::new()));
    let total_tasks = num_tasks;
    let assembled_flag = Arc::new(AtomicBool::new(false));

    let start_time = std::time::Instant::now();

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Worker conectado desde: {:?}", addr);

        let tasks = tasks_arc.clone();
        let results = results_arc.clone();
        let flag = assembled_flag.clone();
        let start = start_time.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_worker(socket, tasks, results, total_tasks as usize, flag, start).await {
                eprintln!("Error manejando worker: {}", e);
            }
        });
    }
}

async fn handle_worker(
    mut socket: TcpStream,
    tasks: Arc<Mutex<Vec<Task>>>,
    results: Arc<Mutex<HashMap<u32, (u32, u32, Vec<u8>)>>>,
    total_tasks: usize,
    assembled_flag: Arc<AtomicBool>,
    start_time: std::time::Instant,
) -> tokio::io::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        let task_opt = {
            let mut tasks = tasks.lock().await;
            tasks.pop()
        };

        let task = match task_opt {
            Some(t) => t,
            None => {
                writer.write_all(b"NO_MORE_TASKS\n").await?;
                writer.flush().await?;
                break;
            }
        };

        println!("Enviando tarea {} (filas {}..{})", task.id, task.y_start, task.y_end);

        let serialized = serde_json::to_string(&task).unwrap() + "\n";
        if let Err(_) = timeout(Duration::from_secs(10), writer.write_all(serialized.as_bytes())).await {
            eprintln!("Timeout enviando tarea {}, reinsertando", task.id);
            tasks.lock().await.push(task);
            continue;
        }
        writer.flush().await?;

        line.clear();
        // Timeout de 240 segundos (4 minutos) para tareas muy pesadas
        let read_result = timeout(Duration::from_secs(240), reader.read_line(&mut line)).await;
        match read_result {
            Ok(Ok(n)) if n > 0 => {
                let result: ResultMsg = match serde_json::from_str(&line) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error parseando resultado de tarea {}: {}", task.id, e);
                        tasks.lock().await.push(task);
                        continue;
                    }
                };

                println!("Resultado recibido de tarea {} (filas {}..{})", result.id, result.y_start, result.y_end);
                {
                    let mut results_map = results.lock().await;
                    results_map.insert(result.id, (result.y_start, result.y_end, result.data));
                    let completed = results_map.len();
                    let elapsed = start_time.elapsed();
                    println!("Progreso: {}/{} tareas completadas, tiempo transcurrido: {:?}", completed, total_tasks, elapsed);

                    if completed == total_tasks && !assembled_flag.load(Ordering::Relaxed) {
                        if assembled_flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                            let results = results.clone();
                            tokio::spawn(async move {
                                if let Err(e) = assemble_image(results, WIDTH, HEIGHT).await {
                                    eprintln!("Error ensamblando imagen: {}", e);
                                } else {
                                    println!("Imagen guardada. Terminando coordinador...");
                                    process::exit(0);
                                }
                            });
                        }
                    }
                }
            }
            Ok(Ok(_)) => {
                eprintln!("Conexión cerrada por worker para tarea {}, reinsertando", task.id);
                tasks.lock().await.push(task);
                break;
            }
            Ok(Err(e)) => {
                eprintln!("Error leyendo resultado de tarea {}: {}", task.id, e);
                tasks.lock().await.push(task);
                break;
            }
            Err(_) => {
                eprintln!("Timeout (240s) esperando resultado de tarea {}, reinsertando", task.id);
                tasks.lock().await.push(task);
            }
        }
    }

    Ok(())
}

async fn assemble_image(
    results: Arc<Mutex<HashMap<u32, (u32, u32, Vec<u8>)>>>,
    width: u32,
    height: u32,
) -> tokio::io::Result<()> {
    let results_map = results.lock().await;
    let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(width, height);

    for (_id, (y_start, y_end, data)) in results_map.iter() {
        let y_start = *y_start;
        let y_end = *y_end;
        let rows = (y_end - y_start) as usize;
        let row_bytes = (width * 3) as usize;
        assert_eq!(data.len(), rows * row_bytes, "Tamaño de datos incorrecto");

        for (i, y) in (y_start..y_end).enumerate() {
            let row_start = i * row_bytes;
            let row_data = &data[row_start..row_start + row_bytes];
            for x in 0..width as usize {
                let r = row_data[x * 3];
                let g = row_data[x * 3 + 1];
                let b = row_data[x * 3 + 2];
                img.put_pixel(x as u32, y, Rgb([r, g, b]));
            }
        }
    }

    img.save("/output/mandelbrot_distributed.png").map_err(|e| {
        eprintln!("Error guardando imagen: {}", e);
        tokio::io::Error::new(tokio::io::ErrorKind::Other, e)
    })?;
    println!("Imagen guardada como '/output/mandelbrot_distributed.png'");

    Ok(())
}