use crate::messages::{Task, ResultMsg};
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{timeout, Duration};
use image::Rgb;

pub async fn run_worker() -> tokio::io::Result<()> {
    // Esperar unos segundos a que el coordinador esté listo
    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut connect_errors = 0;
    const MAX_CONNECT_ERRORS: u32 = 5;

    loop {
        match connect_and_work().await {
            Ok(()) => {
                // connect_and_work retorna Ok cuando recibe NO_MORE_TASKS o cierre normal
                println!("Worker sin tareas, esperando y reconectando...");
                tokio::time::sleep(Duration::from_secs(5)).await;
                connect_errors = 0; // Reiniciamos contador de errores
            }
            Err(e) => {
                eprintln!("Error en worker: {}", e);
                connect_errors += 1;
                if connect_errors >= MAX_CONNECT_ERRORS {
                    eprintln!("Demasiados errores de conexión. Worker termina.");
                    break;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    Ok(())
}

async fn connect_and_work() -> tokio::io::Result<()> {
    let mut stream = TcpStream::connect("coordinator:8080").await?;
    println!("Worker conectado al coordinador");

    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let read_result = timeout(Duration::from_secs(30), reader.read_line(&mut line)).await;
        let line = match read_result {
            Ok(Ok(n)) if n > 0 => line.trim().to_string(),
            Ok(Ok(_)) => {
                println!("Coordinador cerró la conexión");
                return Ok(());
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                eprintln!("Timeout esperando tarea");
                return Err(tokio::io::Error::new(tokio::io::ErrorKind::TimedOut, "Timeout"));
            }
        };

        if line == "NO_MORE_TASKS" {
            println!("Coordinador indica no más tareas. Terminando.");
            return Ok(());
        }

        let task: Task = match serde_json::from_str(&line) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error parseando tarea: {}", e);
                continue;
            }
        };

        println!("Procesando tarea {} (filas {}..{})", task.id, task.y_start, task.y_end);

        // Medir tiempo de cómputo
        let compute_start = std::time::Instant::now();
        let data = compute_mandelbrot_chunk(&task);
        let compute_time = compute_start.elapsed();
        println!("Tarea {} completada en {:?}", task.id, compute_time);

        let result = ResultMsg {
            id: task.id,
            y_start: task.y_start,
            y_end: task.y_end,
            data,
        };

        let serialized = serde_json::to_string(&result).unwrap() + "\n";

        // Medir tiempo de envío
        let send_start = std::time::Instant::now();

        // Bloque de envío (con timeout de 20s como ajustaste)
        if let Err(e) = timeout(Duration::from_secs(20), writer.write_all(serialized.as_bytes())).await {
            eprintln!("Timeout enviando resultado de tarea {}: {}", task.id, e);
            return Err(tokio::io::Error::new(tokio::io::ErrorKind::TimedOut, "Timeout enviando resultado"));
        }
        writer.flush().await?;

        let send_time = send_start.elapsed();
        println!("Tarea {} enviada en {:?}", task.id, send_time);
    }
}

fn compute_mandelbrot_chunk(task: &Task) -> Vec<u8> {
    let width = task.width;
    let height = task.global_height;
    let y_start = task.y_start;
    let y_end = task.y_end;
    let supersampling = task.supersampling;
    let max_iter = task.max_iter;
    let x_min = task.x_min;
    let x_max = task.x_max;
    let y_min = task.y_min;
    let y_max = task.y_max;

    let mut data = Vec::with_capacity(((y_end - y_start) * width) as usize * 3);

    for y in y_start..y_end {
        for x in 0..width {
            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let samples = supersampling * supersampling;

            for sy in 0..supersampling {
                for sx in 0..supersampling {
                    let sub_x = (sx as f64 + 0.5) / supersampling as f64;
                    let sub_y = (sy as f64 + 0.5) / supersampling as f64;

                    let c_re = x_min + ((x as f64 + sub_x) / width as f64) * (x_max - x_min);
                    let c_im = y_min + ((y as f64 + sub_y) / height as f64) * (y_max - y_min);

                    let iter = mandelbrot(c_re, c_im, max_iter);
                    let Rgb([r, g, b]) = iter_to_color(iter, max_iter);
                    r_sum += r as u32;
                    g_sum += g as u32;
                    b_sum += b as u32;
                }
            }

            data.push((r_sum / samples) as u8);
            data.push((g_sum / samples) as u8);
            data.push((b_sum / samples) as u8);
        }
    }
    data
}

fn mandelbrot(c_re: f64, c_im: f64, max_iter: u32) -> u32 {
    let mut z_re = 0.0;
    let mut z_im = 0.0;
    let mut iter = 0;

    while iter < max_iter {
        let z_re_new = z_re * z_re - z_im * z_im + c_re;
        let z_im_new = 2.0 * z_re * z_im + c_im;

        z_re = z_re_new;
        z_im = z_im_new;

        if z_re * z_re + z_im * z_im > 4.0 {
            break;
        }
        iter += 1;
    }
    iter
}

fn iter_to_color(iter: u32, max_iter: u32) -> Rgb<u8> {
    if iter == max_iter {
        Rgb([0, 0, 0])
    } else {
        let t = iter as f64 / max_iter as f64;
        let grad = colorgrad::magma();
        let [r, g, b, _] = grad.at(t).to_rgba8();
        Rgb([r, g, b])
    }
}