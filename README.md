# Formica

## Latar belakang

Terinspirasi dari nama formica yaitu genus dari beberapa spesies semut.

## Todo

* Middleware
* Body Parser
etc

## Benchmark

```
last update: 24/06/2024 UTC+7
```

Baru ada benchmark sederhana dengan routing `GET / `. Benchmark menggunakan `autocannon` dengan CLI sbb: `autocannon http://localhost:9999`
### Source Code
```rust
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
```
### Result
```
Running 10s test @ http://localhost:9999
10 connections


┌─────────┬──────┬──────┬───────┬──────┬─────────┬─────────┬───────┐
│ Stat    │ 2.5% │ 50%  │ 97.5% │ 99%  │ Avg     │ Stdev   │ Max   │
├─────────┼──────┼──────┼───────┼──────┼─────────┼─────────┼───────┤
│ Latency │ 0 ms │ 0 ms │ 0 ms  │ 0 ms │ 0.01 ms │ 0.07 ms │ 12 ms │
└─────────┴──────┴──────┴───────┴──────┴─────────┴─────────┴───────┘
┌───────────┬─────────┬─────────┬────────┬─────────┬───────────┬─────────┬─────────┐
│ Stat      │ 1%      │ 2.5%    │ 50%    │ 97.5%   │ Avg       │ Stdev   │ Min     │
├───────────┼─────────┼─────────┼────────┼─────────┼───────────┼─────────┼─────────┤
│ Req/Sec   │ 27.551  │ 27.551  │ 29.599 │ 29.887  │ 29.410,19 │ 622,39  │ 27.550  │
├───────────┼─────────┼─────────┼────────┼─────────┼───────────┼─────────┼─────────┤
│ Bytes/Sec │ 1.21 MB │ 1.21 MB │ 1.3 MB │ 1.31 MB │ 1.29 MB   │ 27.4 kB │ 1.21 MB │
└───────────┴─────────┴─────────┴────────┴─────────┴───────────┴─────────┴─────────┘

Req/Bytes counts sampled once per second.
# of samples: 11

324k requests in 11.02s, 14.2 MB read
```
