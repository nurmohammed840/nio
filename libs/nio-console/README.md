# nio-console

Nio Console is a terminal-based monitoring tool for Nio async runtime. It provides real-time insights into the performance and behavior of Nio applications.

# Example

```toml
[dependencies]
nio-console = { git = "https://github.com/nurmohammed840/nio" }

# Enable optimization in the dev profile for nio-console.
[profile.dev.package."nio-console"]
opt-level = 3
```

just call `nio_console::launch()` in your application to start the console.
