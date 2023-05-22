# Compilation for Windows (experimental)

Scaphandre, on Windows, needs a kernel driver to get the same metrics as it does with powercap on GNU/Linux. This driver is available on [this repository](https://github.com/hubblo-org/windows-rapl-driver/). Please refer to this project to get either pre-compiled binaries (available soon) or to follow the compilation procedure.

![Scaphandre's dependencies on Windows and GNU/Linux](https://repository-images.githubusercontent.com/421079628/f695abc0-c8e6-46a3-a6f4-6c7c0f617b87)

Once you have a working driver, you can compile Scaphandre, with the Rust for Windows usual toolkit.

For now, all Scaphandre features are not supported on windows. Use the following command line to build the binary :

```
cargo build --no-default-features --features "prometheus json riemann"
```

Don't forget to add the `--release` flag to build a binary suited for more than test and debug usecases.
