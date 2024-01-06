# Tailwind CLI

A library that makes it trivial to invoke the [Tailwind CSS CLI](https://tailwindcss.com/docs/installation) from your Rust code. Useful for running Tailwind in [build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html).

```rust
// build.rs

fn main() {
    // Tell Cargo to rerun this build script if any Rust,
    // CSS, or HTML files change. Make sure this matches
    // the `content` key in your Tailwind config.
    println!("cargo:rerun-if-changed=src/**/*.rs");
    println!("cargo:rerun-if-changed=src/**/*.css");
    println!("cargo:rerun-if-changed=src/**/*.html");

    let args = vec![
        "--input",
        "src/main.css",
        "--output",
        "target/built.css",
    ];
    tailwind_cli::run(&args);
}
```
