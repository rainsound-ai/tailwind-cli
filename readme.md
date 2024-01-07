# Tailwind CLI

A library that makes it trivial to invoke the [Tailwind CSS CLI](https://tailwindcss.com/docs/installation) from your Rust code. Useful for [build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html), among other things.

This crate _doesn't_ include a binary, so it's not a good fit for, say, using `cargo install` to install the Tailwind CLI. If that's a feature you're interested in, please open an issue.

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

    match tailwind_cli::run(&args) {
        Ok(output) => {
            println!("Tailwind CLI completed.");
            println!("stdout:\n{}", output.stdout());
            println!("stderr:\n{}", output.stderr());
        },
        Err(error) => {
            // If we got as far as executing the CLI, `error` will
            // contain the stdout and stderr from the process.
            //
            // If present, they're also included when converting the
            // error to a string.
            println!("Tailwind CLI failed.");
            println!("{}", error);
        },
    }
}
```

## Versioning

Versions of this crate follow the form `v3.4.1-0`, where `3.4.1` is the Tailwind version and `-0` indicates crate versions, in case we need to publish additional crate versions without bumping the Tailwind version.
