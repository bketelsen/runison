# runison 
A tool to synchronize directories between two networked computers

## server
```
RUST_BACKTRACE=1 ./target/debug/runison -d -c /home/bjk/src/github.com/bketelsen/runison/test/server.toml server
```
## client
```
RUST_BACKTRACE=1 ./target/debug/runison -d -c /home/bjk/src/github.com/bketelsen/runison/test/client.toml client -n ghanima
```

## Status

- [x] Create archive of before state of sync directory
- [ ] everything else