# Musikr Fuzzing

Fuzzing can be done with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) if you're running Linux/MacOS:

```
cargo +nightly fuzz run [TARGET]
```

#### Available Targets

```
id3v2 - ID3v2 Fuzzing [Reccomended with -rss_limit_mb=8192mb]
```
