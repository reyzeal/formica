use std::thread::sleep;
use std::time::Duration;
use async_std::{io, task};
use memory_stats::memory_stats;
use formica::{Formica, Response};

#[async_std::main]
async fn main() -> io::Result<()> {
    task::spawn(async {
        loop {
            sleep(Duration::from_secs(1));
            if let Some(usage) = memory_stats() {
                println!("Current physical memory usage: {}", usage.physical_mem as f64 * 0.000001);
            } else {
                println!("Couldn't get the current memory usage :(");
            }
        }
    });
    Formica::new("127.0.0.1:9999")
        .post("/",  |x, mut y | {
            y.body(String::from_utf8_lossy(x.content).to_string());
            y
        })
        .get("/",  |x, mut y | {
            y.body("OK GET".to_string());
            y
        })
        .listen().await?;
    Ok(())
}