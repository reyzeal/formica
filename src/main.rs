use async_std::{io};
use formica::{Formica, Response};

#[async_std::main]
async fn main() -> io::Result<()> {
    Formica::new("127.0.0.1:9999")
        .post("/",  |x, mut y | {
            y.body("OK POST".to_string());
            y
        })
        .get("/",  |x, mut y | {
            y.body("OK GET".to_string());
            y
        })
        .listen().await?;
    Ok(())
}