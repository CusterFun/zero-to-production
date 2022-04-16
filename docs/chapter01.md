# 开始

## 创建项目

`cargo new zero2prod`

## Faster Linking

A sizeable chunk of time is spent in the [Linking phase](<https://en.wikipedia.org/wiki/Linker_(computing)>) - assembling the actual binary given the outputs of the earlier compilation stages.

The default linker does a good job, but there are faster alternatives depending on the operating system you are using:

- lld on Windows and Linux, a linker developed by the LLVM project;
- zld on MacOS.

To speed up the linking phase you have to install the alternative linker on your machine and add this configuration file to the project:

````toml
# .cargo/config.toml
# On Windows
# ```
# cargo install -f cargo-binutils
# rustup component add llvm-tools-preview
# ```
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
# On Linux:
# - Ubuntu, `sudo apt-get install lld clang`
# - Arch, `sudo pacman -S lld clang`
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "linker=clang", "-C", "link-arg=-fuse-ld=lld"]
# On MacOS, `brew install michaeleisel/zld/zld`
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
````

There is [ongoing work](https://github.com/rust-lang/rust/issues/39915#issuecomment-618726211) on the Rust compiler to use lld as the default linker where possible - soon enough this custom configuration will not be necessary to achieve higher compilation performance!

## 安装 cargo-watch

```shell
cargo install cargo-watch
```

cargo-watch monitors your source code to trigger commands every time a file changes. For example:

```shell
cargo watch -x check
```

cargo-watch 也支持链式调用，比如

```shell
cargo watch -x check -x text -x run
```

该命令会先运行 `cargo check` 如果成功了，会继续运行 `cargo test`，如果测试成功，会继续运行 `cargo run`

## Continuous Integration

安装好 rust 工具链，创建了项目，IDE 准备好了，最后一步就是 CI 集成。

CI 的第一步是

### 1.`cargo test`

### 2. Code Coverage

使用代码覆盖率作为质量检查，测量 Rust 项目的代码覆盖率最简单的方法是通过 `cargo tarpaulin`。

```shell
# At the time of writing tarpaulin only supports
# x86_64 CPU architectures running Linux.
cargo install cargo-tarpaulin
```

运行以下命令忽略测试函数，计算代码的测试覆盖率，并可以上传覆盖率指标到 codecov 或 coveralls 流行的服务。

```shell
cargo tarpaulin --ignore-tests
```

### 3. linting

The Rust team maintains [clippy](https://github.com/rust-lang/rust-clippy), the official Rust linter

```shell
rustup component add clippy
```

运行

```shell
cargo clippy
```

在 CI 管道中，如果 clippy 发出任何警告，我们希望 linter 检查失败。可以通过以下方式实现

```shell
cargo clippy -- -D warnings
```

使用以下属性 `#[allow(clippy::lint_name)]` 在受影响的代码上可以忽略该警告

或者项目级 `#![allow(clippy::lint_name)]` 忽略该文件的警告

### 4. Formatting

安装

```shell
rustup component add rustfmt
```

使用命令格式化整个项目

```shell
cargo fmt
```

在 CI 中添加格式化步骤

```shell
cargo fmt -- --check
```

当一个提交包含未格式化的代码时，它将失败，并将差异打印到控制台，可以配置 `rustfmt.toml` 为项目调整 `rustfmt`。

### 5. Security Vulnerabilities

Rust Secure Code 工作组维护一个 [Advisory Database](https://github.com/RustSec/advisory-db) 。他们还提供了一个 `cargo-audit`。

检查你的项目依赖中的 crate 是否有漏洞被报告。

```shell
cargo install cargo-audit
```

运行命令检查

```shell
cargo audit
```

添加到 CI 的管道一部分，在每次提交时运行。以保持对项目依赖 crate 的新漏洞的关注。

### 6. 使用 GitHub Actions

https://gist.github.com/LukeMathWalker/5ae1107432ce283310c3e601fac915f3

添加文件夹 `.github/workflows`

```yaml
# .github/workflows/audit-on-push.yml

name: Security audit
on:
  push:
    paths:
      - '**/Cargo.toml'
      - '**/Cargo.lock'
jobs:
  security_audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v1
      - uses: actions-rs/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

```yaml
# .github/workflows/general.yml
name: Rust

on: [push, pull_request]

env:
  CARGO_TERM_COLOR: always

jobs:
  # cargo test
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      - uses: actions-rs/cargo@v1
        with:
          command: test

  # cargo fmt --all -- --check
  fmt:
    name: Rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
          components: rustfmt
      - uses: actions-rs/cargo@v1
        with:
          command: fmt
          args: --all -- --check

  # cargo clippy -- -D warnings
  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
          components: clippy
      - uses: actions-rs/clippy-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          args: -- -D warnings

  # cargo tarpaulin --ignore-tests
  coverage:
    name: Code coverage
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v2

      - name: Install stable toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Run cargo-tarpaulin
        uses: actions-rs/tarpaulin@v0.1
        with:
          args: "--ignore-tests"
```

```yaml
# .github/workflows/scheduled-audit.yml

name: Security audit
on:
  schedule:
    - cron: '0 0 * * *'
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v1
      - uses: actions-rs/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

