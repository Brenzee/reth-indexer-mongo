# Reth Log Indexer with MongoDB

Easily collect and index events directly from the Reth database into a MongoDB. Inspired from reth-indexer.

## Benchmarks

| Event            | Block from | Block to | Amount of docs | Time taken         |
| ---------------- | ---------- | -------- | -------------- | ------------------ |
| Uniswap V2 pairs | 10000835   | 21180626 | 388095         | 565.56s (9min 25s) |
| Uniswap V3 pools | 12369621   | 21180626 | 28856          | 149.48s (2min 29s) |
