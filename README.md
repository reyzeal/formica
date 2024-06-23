# Formica

## Latar belakang

Terinspirasi dari nama formica yaitu genus dari beberapa spesies semut.

## Todo

* Middleware
* Body Parser
etc

## Benchmark
Baru ada benchmark sederhana dengan routing `GET / `. Benchmark menggunakan `autocannon` dengan CLI sbb: `autocannon http://localhost:9999`
### Source Code
```rust
#[async_std::main]
async fn main() -> io::Result<()> {
    Formica::new("127.0.0.1:9999")
        .post("/",  |x, mut y | {
            y.body("OK POST".to_string());
            y
        }).await
        .get("/",  |x, mut y | {
            y.body("OK GET".to_string());
            y
        }).await
        .listen().await?;
    Ok(())
}
```
### Result
```
Running 10s test @ http://localhost:9999
10 connections


┌─────────┬──────┬──────┬───────┬──────┬─────────┬─────────┬───────┐
│ Stat    │ 2.5% │ 50%  │ 97.5% │ 99%  │ Avg     │ Stdev   │ Max   │
├─────────┼──────┼──────┼───────┼──────┼─────────┼─────────┼───────┤
│ Latency │ 0 ms │ 0 ms │ 0 ms  │ 0 ms │ 0.01 ms │ 0.08 ms │ 13 ms │
└─────────┴──────┴──────┴───────┴──────┴─────────┴─────────┴───────┘
┌───────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐
│ Stat      │ 1%      │ 2.5%    │ 50%     │ 97.5%   │ Avg     │ Stdev   │ Min     │
├───────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤
│ Req/Sec   │ 25.759  │ 25.759  │ 27.135  │ 27.695  │ 27.096  │ 494,2   │ 25.747  │
├───────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤
│ Bytes/Sec │ 1.13 MB │ 1.13 MB │ 1.19 MB │ 1.22 MB │ 1.19 MB │ 21.7 kB │ 1.13 MB │
└───────────┴─────────┴─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘

Req/Bytes counts sampled once per second.
# of samples: 10

271k requests in 10.02s, 11.9 MB read
```
