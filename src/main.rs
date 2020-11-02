use clap::{App, Arg};

use std::str::FromStr;

use opencv::{
    prelude::*,
    imgproc::{cvt_color, COLOR_BGR2GRAY},
    highgui::{imshow, wait_key},
    videoio::{VideoCapture, CAP_ANY}
};

fn detect_and_display (frame: Mat) {
    let mut frame_gray = Mat::default().expect("Failed to allocate new frame!");
    cvt_color(&frame, &mut frame_gray, COLOR_BGR2GRAY, 0).expect("Failed to recolor frame!");
    
    imshow("Video", &frame_gray).expect("Failed to create output window!");
}

fn main() {
    let matches = App::new("FaceStuff")
        .version("0.1.0")
        .author("Jack Duvall <jrduvall@andrew.cmu.edu>")
        .about("OpenCV with Rust Face Detection Tests")
        .arg(
            Arg::with_name("camera_id")
                .short("c")
                .long("camera_id")
                .value_name("INT")
                .help("Selects with camera index to use (Default: 0)")
                .takes_value(true),
        )
        .get_matches();

    let camera_id_arg = matches.value_of("camera_id").unwrap_or("0");
    let camera_id = i32::from_str(camera_id_arg).unwrap_or(0);
    println!("Opening camera {}", camera_id);

    let mut cap = VideoCapture::new(camera_id, CAP_ANY).expect("Failed opening camera!");

    let mut frame = Mat::default().expect("Failed to allocate memory for frame!");
    while cap.read(&mut frame).expect("Failed to read frame!") {
        let frame_to_show = Mat::copy(&frame).expect("Failed to copy frame!");
        detect_and_display(frame_to_show);
        
        if wait_key(24).expect("Failed to read keypress!") >= 0 {
            println!("Exiting...");
            break;
        }
    }
}
