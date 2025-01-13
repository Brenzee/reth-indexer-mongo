# Reth Log Indexer with MongoDB

Easily collect and index events directly from the Reth database into MongoDB. Inspired from [reth-indexer](https://github.com/joshstevens19/reth-indexer)

## Benchmarks

| Event            | Block from | Block to | Amount of docs | Time taken         |
| ---------------- | ---------- | -------- | -------------- | ------------------ |
| Uniswap V2 Pairs | 10000835   | 21614513 | 398152         | 558s (9min 18s)    |
| Uniswap V3 Pools | 12369621   | 21614513 | 29919          | 145s (2min 25s) |
