use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: u32,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub width: u32,
    pub global_height: u32,
    pub y_start: u32,
    pub y_end: u32,
    pub max_iter: u32,
    pub supersampling: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResultMsg {
    pub id: u32,
    pub y_start: u32,
    pub y_end: u32,
    pub data: Vec<u8>, // Datos RGB en orden de filas (y_start..y_end), cada píxel 3 bytes
}