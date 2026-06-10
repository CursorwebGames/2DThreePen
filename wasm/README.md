## Build
```sh
# dev
wasm-pack build wasm --target web --dev

# prod
wasm-pack build wasm --target web
```

Useful Links:

* [mdn](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm)
* [quickstart](https://wasm-bindgen.github.io/wasm-pack/book/quickstart.html)
* `wasm-pack new [project]`

## Decisions
[AlgoX Wikipedia](https://en.wikipedia.org/wiki/Knuth%27s_Algorithm_X)
**Constraints**: Each row needs one bull, each column needs one bull, each region needs one bull (3n)
**Candidates**: Single bull (n^2)

Also: When you place a bull, also remove adjacent bull candidates in the matrix

## References
[DLX](https://ferrous-systems.com/blog/dlx-in-rust/)