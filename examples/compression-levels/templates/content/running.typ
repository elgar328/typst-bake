The next page shows benchmark results. To run them and generate this PDF on your own machine:

```bash
cargo test -p example-compression-levels --release --test benchmark -- --ignored --nocapture --test-threads=1
cargo run -p example-compression-levels
```
