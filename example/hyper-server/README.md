# Run Server

```
cd ./example
❯ cargo run -r --example hyper-server
❯ cargo run -r -F tokio --example hyper-server
```

# Run Bench

```
pip install matplotlib
❯ python3 ./hyper-server/bench.py --duration 3 --prefix hyper
```

## Run wrk

```
❯ wrk -t4 -c100 -d3 http://127.0.0.1:4000
```