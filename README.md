# 学习用虚拟机

## Quick Testing Guide for qhyper

**Prerequisites**

Install the `Rust Targets` extension in VSCode

Install the `ostool` utility:

``` shell
cargo install ostool
```

**Setup**

Clone the repository and navigate to the project directory:

``` shell
git clone https://github.com/qclic/qhyper.git
cd qhyper
```

Configure the project:

``` shell
mv ./.project.toml.tmp ./.project.toml
```

In VSCode, use the Rust Targets extension to select your target architecture:

+ For aarch64 debugging: Set target to `aarch64-unknown-none`
+ For riscv64 debugging: Set appropriate riscv64 target

**Debugging**

Choose the appropriate debug configuration based on your selected architecture:

+ For aarch64: Select "KDebug lldb"
+ For riscv64: Select "KDebug cppdbg"

Press F5 in VSCode to start debugging. The execution will automatically pause at the first line of code running in QEMU.

From there, you can:

+ Set breakpoints in VSCode
+ Use VSCode's debugging features to step through code
+ Inspect variables and program state

now. enjoy it!