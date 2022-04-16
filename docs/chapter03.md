[toc]

## 注册邮件订阅

作为一个博客访问者，我留下了自己的邮箱订阅博客的更新。

该表单的提交调用后台服务器的 API，该 API 处理提交的 email 信息，存储并返回响应。

## 前期工作

从头开始一个新的项目，有相当多的前期工作需要处理

- 选择一个熟悉的 Web 框架
- 确定测试的策略
- 选择 crate 与数据库交互
- 定义数据库表结构和后续数据库迁移管理
- 实际写接口代码

首先实现一个 /health_check 接口，通过 [CI 模板](https://gist.github.com/LukeMathWalker/5ae1107432ce283310c3e601fac915f3) 来走通开发流程。

## 添加 GitHub Actions

https://gist.github.com/LukeMathWalker/5ae1107432ce283310c3e601fac915f3

添加文件夹 `.github/workflows`

```yaml
# .github/workflows/audit-on-push.yml

name: Security audit
on:
  push:
    paths:
      - "**/Cargo.toml"
      - "**/Cargo.lock"
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
    - cron: "0 0 * * *"
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v1
      - uses: actions-rs/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

## `actix-web`

- [actix-web 官网](https://actix.rs/)
- [actix-web docs 文档](https://docs.rs/actix-web)
- [actix-web example](https://github.com/actix/examples)

## `/health_check` 接口

使用 `GET /health_check` 接口可以检查服务器的状态，返回值为 `200 OK`。

### 使用 `actix-web`

复制 `actix-web` 主页的示例代码到 `main.rs` 中

```rust
use actix_web::{web, App, HttpRequest, HttpServer, Responder};
async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}
#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
```

在 `Cargo.toml` 中手动添加 `actix-web` 和 `tokio` 依赖

```toml
[dependencies]
actix-web = "4"
tokio = {version = "1", features = ["macros", "rt-multi-thread"]}
```

或者可以使用 `cargo add` 快速添加这两个依赖的最新版本

```shell
cargo add actix-web@4
```

`cargo add` 不是默认的 `cargo` 命令，需要安装 `cargo-edit`

```shell
cargo install cargo-edit
```

此时运行 `cargo check` 应该没有错误，使用 `cargo run` 启动应用程序。并执行命令手动测试：

```shell
curl http://127.0.0.1:8000
```

可以看到返回的

```shell
Hello World!
```

可以使用 `Ctrl + C` 关闭 `web` 服务器。

### 剖析 `actix-web` 应用程序

现在回过头来仔细看看复制粘贴到 `main.rs` 文件中的内容

```rust
//! src/main.rs
// [...]

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
```

#### Server - `HttpServer`

[`HttpServer`](https://docs.rs/actix-web/4.0.1/actix_web/struct.HttpServer.html) 是应用程序的主干，它负责以下事项：

- 应用程序应该在哪里监听传入的请求？一个 TCP Socket（如：127.0.0.1：8000）？一个 Unix domain socket?
- 我们应该允许的最大并发连接数是多少？单位时间内有多少新链接？
- 我们应该启动 TLS 吗？
- 等等。

换句话说， `HttpServer` 处理所有传输层的问题。

#### Application - `App`

应用程序 [`App`](https://docs.rs/actix-web/4.0.1/actix_web/struct.App.html) 是所有应用程序的逻辑：`routing`，`middlewares`，`request handlers` 等。

`App` 是一个组件，它的工作是将一个传入的请求作为输入，并返回一个响应。

```rust
App::new()
    .route("/", web::get().to(greet))
    .route("/{name}", web::get().to(greet))
```

`App` 是 `builder pattern` 的应用，可以将 `API` 链式调用。

#### Endpoint - Route

[`route method`](https://docs.rs/actix-web/4.0.1/actix_web/struct.App.html#method.route)

```rust
pub fn route(self, path: &str, route: Route) -> Self
```

Configure route for a specific path.

This is a simplified version of the `App::service()` method. This method can be used multiple times with same path, in that case multiple resources with one route would be registered for same resource path.

有两个参数

- `path` 一个字符串，可能是模板化的(如："/{name}")以动态加载路径
- `route` `Route` 结构体的实例

[`Route struct`](https://docs.rs/actix-web/4.0.1/actix_web/struct.Route.html) combines a *handler* with a set of *guards*

```rust
/// A request handler with [guards](guard).
///
/// Route uses a builder-like pattern for configuration. If handler is not set, a `404 Not Found`
/// handler is used.
pub struct Route {
    service: BoxedHttpServiceFactory,
    guards: Rc<Vec<Box<dyn Guard>>>,
}
```

Route 将一个处理程序与一组 *guards* 结合起来。

Guards 指定了请求必须满足的条件，以便“匹配”并传递给处理程序，

具体的实现是通过 [`Guard trait`](https://docs.rs/actix-web/4.0.1/actix_web/guard/trait.Guard.html)

**Trait [`actix_web`](https://docs.rs/actix-web/4.0.1/actix_web/index.html)::[guard](https://docs.rs/actix-web/4.0.1/actix_web/guard/index.html)::[Guard](https://docs.rs/actix-web/4.0.1/actix_web/guard/trait.Guard.html#)**

```rust
pub trait Guard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool;
}
```

Interface for routing guards. See [module level documentation](https://docs.rs/actix-web/4.0.1/actix_web/guard/index.html) for more.

查看我们的代码

```rust
.route("/", web::get().to(greet))
```

"/" 将匹配"/"路径后没有任何字段的所有请求。如 http://localhost:8000/

`web.get()` 是 `Route::new().guard(guard::Get())` 的快捷方式，当且仅当请求的 `HTTP` 方法是 `GET` 时，才应该将请求传递给处理程序。

`handler` 处理程序的函数

```rust
async fn greet(req: HttpRequest) -> impl Responder {
    [...]
}
```

**greet** 是一个异步函数，它接收一个 `HttpRequest` 作为输入，并返回一个实现了 [`Responder trait`](https://docs.rs/actix-web/4.0.1/actix_web/trait.Responder.html) 具体类型。

**Trait [`actix_web`](https://docs.rs/actix-web/4.0.1/actix_web/index.html)::[Responder](https://docs.rs/actix-web/4.0.1/actix_web/trait.Responder.html#)**

```rust
pub trait Responder {
    type Body: MessageBody + 'static;
    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body>;

    fn customize(self) -> CustomizeResponder<Self>
    where
        Self: Sized,
    { ... }
}
```

> Trait implemented by types that can be converted to an HTTP response.
>
> Any types that implement this trait can be used in the return type of a handler. Since handlers will only have one return type, it is idiomatic to use opaque return types `-> impl Responder`.

如果一个类型可以被转成 `HttpResponse`，那么它就实现了 `Responder trait`。

常见的 `strings`，`status codes`，`bytes`，`HttpResponse` 等都实现了 `Responder trait`。

如果需要，我们也可以针对自己的类型实现 `Responder trait`。

#### Runtime - tokio

```rust
#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
```

`HttpServer::run` 是一个异步函数，**Rust** 的异步编程构建在 [`Future trait`](https://doc.rust-lang.org/beta/std/future/trait.Future.html)，

> a future stands for a value that may not be there *yet*. All futures expose a [poll method](https://doc.rust-lang.org/beta/std/future/trait.Future.html#the-poll-method) which has to be called to allow the future to make progress and eventually resolve to its final value.

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

你可以认为 Rust 的 *futures* 是懒惰的，除非轮询（unless polled），否则不能保证它们会执行到完成。与其他百年城语言采取的 **push model** 相比，这通常被称为 **pull model**。

根据设计 Rust 标准库中不包括异步运行时，你可以将任何一个异步运行时作为依赖引入到项目中，以满足实际的特定需求（如 [Fuchsia project](http://smallcultfollowing.com/babysteps/blog/2019/12/09/async-interview-2-cramertj/#async-interview-2-cramertj) 或者 [bastion&#39;s actor](https://github.com/bastion-rs/bastion) 框架）。

在 `main` 函数的顶部启动[异步运行时 allocators](https://doc.rust-lang.org/1.9.0/book/custom-allocators.html) `#[tokio::main]`，用它来驱动 `future`。

`tokio::main` 是一个 `procedural macro`，我们可以使用 `cargo expand`，来查看展开的代码

```shell
cargo install cargo-expand
```

**Rust** 宏的主要目的是生成代码。

我们使用的是 `stable` 编译器来构建、测试和运行我们的代码，但是 `cargo-expand` 需要使用 `nightly` 的编译器来扩展宏代码，可以通过以下命令来安装 `nightly` 编译器

```shell
rustup toolchain install nightly --allow-downgrade
```

`--allow-downgrade` 告诉 `rustup` 编译器在所有需要的组件都可用的情况下查找并安装最新版的 `nightly` 版本。

可以使用 `rustup default` 来指定 `cargo` 和 `rustup` 管理的其他工具所使用的默认工具链。在这里我们只是需要用它来扩展宏代码，不需要切换到 `nightly`，可以使用如下命令指定工具链使用的版本

```shell
# Use the nightly toolchain just for this command invocation
cargo +nightly expand
```

我们可以看到宏展开后的代码

```rust
fn main() -> std::io::Result<()> {
    let body = async {
        HttpServer::new(|| {
            App::new()
                .route("/", web::get().to(greet))
                .route("/{name}", web::get().to(greet))
        })
        .bind("127.0.0.1:8000")?
        .run()
        .await
    };
    #[allow(clippy::expect_used)]
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body)
}
```

在 `#[tokio::main]` 展开后传递给 Rust 编译器的 main 函数确实是同步的。

```rust
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body)
```

我们启动 `tokio` 异步运行时，用它来驱动 `future` 等待 `HttpServer::run` 运行完成的返回。

换句话说，`#[tokio::main]` 是给我们一种能够定义异步 main 函数的错觉，而在幕后，它只是获取了 main 函数主体，并编写了必要的模板代码，使其在 `tokio` 的运行时之上运行。

## 实现健康检查接口

通过上面的 *Hello World!* 示例，已经了解到了 `HttpServer`、`App`、`route`、`tokio::main`、`Responder`

```rust
async fn health_check(req: HttpRequest) -> impl Responder {
    todo!()
}
```

前面说过 `Responder` 是一个到 `HttpResponse` 的 `trait` 转换。所以直接返回 `HttpResponse` 的一个实例应该就可以了。

查看[文档](https://docs.rs/actix-web/4.0.1/actix_web/struct.HttpResponse.html)，我们可以使用 `HttpResponse::Ok` 来获取一个 [`HttpResponseBuilder`](https://docs.rs/actix-web/4.0.1/actix_web/struct.HttpResponseBuilder.html)  200 状态码。

```rust
/// An outgoing response.
pub struct HttpResponse<B = BoxBody> {
    res: Response<B>,
    error: Option<Error>,
}
```

可以使用 `HttpResponseBuilder` 来逐步构建 `HttpResponse` 响应。但是这里我们并不需要它。

```rust
/// An HTTP response builder.
///
/// This type can be used to construct an instance of `Response` through a builder-like pattern.
pub struct HttpResponseBuilder {
    res: Option<Response<BoxBody>>,
    error: Option<HttpError>,
}
```

这里我们可以通过调用 [`finish`]() 来返回

```rust
    /// Set an empty body and build the `HttpResponse`.
    ///
    /// `HttpResponseBuilder` can not be used after this call.
    #[inline]
    pub fn finish(&mut self) -> HttpResponse {
        self.body(())
    }
```

```rust
async fn health_check(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().finish()
}
```

对 `HttpResponseBuilder` 进一步研究发现，它也实现了 `Responder`，因此我们可以省略对 `finish` 的调用。

```rust
async fn health_check(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
}
```

下一步是对 *handler* 的注册，我们通过 `route` 将 `handler` 添加到应用程序

```rust
App::new()
	.route("/health_check", web::get().to(health_check))
```

让我们看下整体的全貌

```rust
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};

async fn health_check(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .bind("127.0.0.1:8000")?
        .run()
        .await
}
```

运行 `cargo check` ，它提出了一个警告

```shell
warning: unused variable: `req`
 --> src\main.rs:3:23
  |
3 | async fn health_check(req: HttpRequest) -> impl Responder {
  |                       ^^^ help: if this is intentional, prefix it with an underscore: `_req`
  |
  = note: `#[warn(unused_variables)]` on by default
```

我们可以遵循编译器的建议，在 `req` 前面加一个下划线，或者可以从 `health_check` 中删除这个输入参数

```rust
async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}
```

运行程序 `cargo run`

新打开一个命令行程序，运行

```shell
curl -v http://127.0.0.1:8000/health_check 
```

可以看到返回 200，到这里就完成了第一个 `actix-web` 的接口实现。

## 第一个集成测试

`health_check` 是我们的第一个接口，我们通过启动应用程序并通过 *curl* 手动测试，验证了一切正常。

然而手动测试是耗时的，随着我们的应用程序变得越来越大，每次我们执行一些更改时，手动测试的成本也越来越高。我们希望尽可能地自动化：我们每次提交变更时，这些检查应该在我们的 CI 中运行。

### 如何测试一个接口

黑盒测试：在给定一组输入的情况下，我们通过检查其输出来验证系统的行为，而无需访问其内部实现的细节。遵循这个原则，我们不会满足于直接调用 *handler* 函数的测试，如下：

```rust
use actix_web::{web, App, HttpResponse, HttpServer};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .bind("127.0.0.1:8000")?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use crate::health_check;

    #[tokio::test]
    async fn health_check_successed() {
        let response = health_check().await;
        // This requires changing the return type of `health_check`
        // from `impl Responder` to `HttpResponse` to compile
        // You also need to import it with `use actix_web::HttpResponse`!
        assert!(response.status().is_success());
    }
}
```

因为我们还没有检查 **GET** 请求是否调用了 *handler*，还没有检查 *handler* 是否以 `/health_check` 作为路由被调用。

改变上面这两个属性中的任何一个都会破坏我们的 **`Api`** 契约，但是我们的测试仍然会通过。

actix-web 提供了一些[便利的方式](https://actix.rs/docs/testing/)与应用程序交互，但这种方法有严重的缺点：

- 迁移到另一个 web 框架会迫使我们重写整个集成测试。尽可能的，我们希望集成测试和支撑我们 `API` 实现的技术分离。
- 由于 actix-web 的一些限制，我们无法在生产代码和测试代码之间共享我们的应用程序启动逻辑。

这里选择完全黑盒的方式：我们将在每次测试开始时启动我们的应用程序，并使用现在的 HTTP 客户端(例如：[reqwest](https://crates.io/crates/reqwest))

### 测试代码放在哪里

Rust 给我们三种选择来编写测试

- 在代码中插入测试模块，如：

```rust
// Some code I want to test
#[cfg(test)]
mod tests {
    // Import the code I want to test
    use super::*;
    // My tests
}
```

- 在外部的 `tests` 目录下

```shell
> ls
src/
tests/
Cargo.toml
Cargo.lock
...
```

- 作为文档的一部分（文档测试），如

```rust

// Check if a number is even.
/// ```rust
/// use zero2prod::is_even;
///
/// assert!(is_even(2));
/// assert!(!is_even(1));
/// ```
pub fn is_even(x: u64) -> bool {
    x % 2 == 0
}
```

它们之间的区别：

代码中的测试模块是项目的一部分，只是隐藏在[配置条件 configuration conditional check](https://doc.rust-lang.org/stable/rust-by-example/attribute/cfg.html)下，相反 **tests** 文件夹下的任何内容和文档测试都是在它们自己单独的二进制文件中编译。

代码测试模块是由特权访问它旁边的代码，它可以与未被标记为公共的结构、方法、字段和函数进行交互。针对暴露公共接口非常有限，底层有大量的复杂逻辑处理，通过暴露的函数来测试所有可能的边缘情况有可能不是那么直截了当，通过嵌入代码测试模块来为私有子组件编写单元测试，以增加你对整个项目正确性的信心。

相反，外部 tests 文件夹中的测试和文档测试对代码的访问级别于你将 crate 作为一个依赖项添加到另一个项目中所获得的访问级别完全相同。因此它们主要用于集成测试，也就是说，以与用户完全相同的方式调用代码来测试代码。

### 改变项目结构以便测试

创建一个新的文件 `tests/health_check.rs`

```shell
# Create the tests folder
mkdir -p tests
```

创建一个新的文件

```rust
//! tests/health_check.rs
use zero2prod::main;
#[test]
fn dummy_test() {
    main()
}
```

我们将项目重构为一个库和一个二进制文件，我们所有的逻辑都将放在 library crate 中，而二进制文件作为入口，具有非常小的 main 函数。

```toml
[package]
edition = "2021"
name = "zero2prod"
version = "0.1.0"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
actix-web = "4"
tokio = {version = "1", features = ["macros", "rt-multi-thread"]}
```

创建 `src/lib.rs` 文件，修改 `Cargo.toml` 文件为如下

```toml
[package]
edition = "2021"
name = "zero2prod"
version = "0.1.0"

[lib]
path = "src/lib.rs"

# Notice the double square brackets: it's an array in TOML's syntax.
# We can only have one library in a project, but we can have multiple binaries!
# If you want to manage multiple libraries in the same repository
[[bin]]
name = "zero2prod"
path = "src/main.rs"

[dependencies]
actix-web = "4"
tokio = {version = "1", features = ["macros", "rt-multi-thread"]}
```

接下来修改二进制文件 `main.rs` 入口函数

修改之前的样子

```rust
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .bind("127.0.0.1:8000")?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use crate::health_check;

    #[tokio::test]
    async fn health_check_successed() {
        let response = health_check().await;
        // This requires changing the return type of `health_check`
        // from `impl Responder` to `HttpResponse` to compile
        // You also need to import it with `use actix_web::HttpResponse`!
        assert!(response.status().is_success());
    }
}
```

```rust
//! main.rs

use zero2prod::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    run().await
}
```

修改 `lib.rs`

```rust
//! lib.rs
use actix_web::{web, App, HttpResponse, HttpServer};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// We need to mark `run` as public.
// It is no longer a binary entrypoint, therefore we can mark it as async
// without having to use any proc-macro incantation.
pub async fn run() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .bind("127.0.0.1:8000")?
        .run()
        .await
}
```

### 实现第一个集成测试

```rust
//! tests/health_check.rs

// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)
#[tokio::test]
async fn health_check_works() {
    // Arrange
    spawn_app().await.expect("Failed to spawn our app.");
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();
    // Act
    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

// Launch our application in the background ~somehow~
async fn spawn_app() -> std::io::Result<()> {
    todo!()
}
```

运行 `cargo-edit` 命令添加 `reqwest` 依赖到开发依赖下

```shell
cargo add reqwest@0.11 --dev
```

花点时间查看上面的测试代码

`spawn_app` 是唯一一个合理地依赖于我们应用程序代码地部分，其他一切都与底层实现细节完全分离。

如果之后使用 Go 语言或者 axum 框架重写应用程序，我们仍然可以使用相同地测试来检查应用程序地功能。

该测试还涵盖了我们需要检查地所有属性：

- 运行状况检查在 `/health_check` 路由访问
- 运行状况检查是通过 **GET** 方法请求
- 健康检查总是返回 200
- 运行状况检查地响应没有正文

添加

```rust
//! tests/health_check.rs
// [...]

async fn spawn_app() -> std::io::Result<()> {
    zero2prod::run().await
}
```

运行命令

```rust
cargo test
```

无论等待多长时间，测试执行永不会终止。在 `zero2prod::run` 中我们调用并等待 `HttpServer::run`。

当我们调用时，`HttpServer::run` 返回一个 `Server` 的一个实例。`await` 它开始监听指定的地址，收到请求时处理这些请求，但它永远不会自行关闭或”完成“。

这意味着 `spawn_app` 永远不会返回，我们的测试逻辑永远不会执行。我们需要将应用程序作为后台任务运行。

这里使用 [`tokio::spawn`](https://docs.rs/tokio/latest/tokio/fn.spawn.html) 获取一个 `future`，并将其交给运行时进行轮询，而不等待其完成。

```rust
pub fn spawn<T>(future: T) -> JoinHandle<T::Output>ⓘ 
where
    T: Future + Send + 'static,
    T::Output: Send + 'static, 
```

需要相应的重构 `zero2prod::run` 来返回一个服务而不等待它结束

```rust
//! lib.rs
use actix_web::{dev::Server, web, App, HttpResponse, HttpServer};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// We need to mark `run` as public.
// It is no longer a binary entrypoint, therefore we can mark it as async
// without having to use any proc-macro incantation.
pub fn run() -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .bind("127.0.0.1:8000")?
        .run();
    // No .await here!
    Ok(server)
}
```

修改 `src/main.rs`

```rust
//! src/main.rs

use zero2prod::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    run()?.await
}
```

运行 `cargo check` 一切正常，现在来修改 `spawn_app` 函数

```rust
//! tests/health_check.rs

// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)
#[tokio::test]
async fn health_check_works() {
    // No .await, no .expect
    spawn_app();
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();
    // Act
    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

// No .await call, therefore no need for `spawn_app` to be async now.
// We are also running tests, so it is not worth it to propagate errors:
// if we fail to perform the required setup we can just panic and crash
// all the things.
fn spawn_app() {
    let server = zero2prod::run().expect("Failed to bind address");
    // Launch the server as a background task
    // tokio::spawn returns a handle to the spawned future,
    // but we have no use for it here, hence the non-binding let
    let _ = tokio::spawn(server);
}
```

运行 `cargo test` 一切正常，第一个集成测试完成了。

## 修正 Polishing

现在程序运行正常，我们回过头来看看有的地方是否可以改进的更好。

### Clean Up

当测试运行结束时，在后台运行的应用程序会正常关闭吗？8000 端口会被占用吗？

连续多次运行 `cargo test` 总是成功的，看上去在每次运行结束时，应用程序被正确关闭了。

再看以下 `tokio::spawn` 的文档，可以支持我们的假设，当 `tokio` 运行时关闭时，其上生成的所有任务都会被丢弃。`tokio::test` 在每个测试用例开始时启动一个新的运行时，在每个测试用例结束时关闭。

意味着，我们不需要实现任何清理逻辑来避免运行结束时的资源泄露。

### 选择随机端口

`spawn_app` 总是在端口 8000 上运行我们的程序，这并不理想

- 如果机器上的另一个程序正在使用端口8000，测试会失败
- 如果尝试并行运行两个或更多测试，其中只有一个能够绑定端口，所有其他测试都将失败

我们可以做的更好：测试应该在随机可用的端口上运行后台应用程序。

手写我们修改 `run` 函数，它应该将地址作为参数，而不是依赖硬编码的值。

```rust
//! src/lib.rs
use actix_web::{dev::Server, web, App, HttpResponse, HttpServer};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(address: &str) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .bind(address)?
        .run();
    Ok(server)
}
```

相应的修改所有 `zero2prod::run()` 调用，更改为 `zero2prod::run("127.0.0.1:8000")`

```rust
//! src/main.rs

use zero2prod::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    run("127.0.0.1:8000")?.await
}
```

运行 `cargo check` 一切正常，继续修改

我们如何为测试找到一个随机可用的端口？

操作系统提供了 [port 0](https://www.lifewire.com/port-0-in-tcp-and-udp-818145)，尝试绑定端口 0 将触发操作系统扫描可用端口，然后该端口将被绑定到应用程序。因此将 `spawn_app` 更改为：

```rust
//! tests/health_check.rs
// [...]

fn spawn_app() {
    let server = zero2prod::run("127.0.0.1:0").expect("Failed to bind address");
    let _ = tokio::spawn(server);
}
```

现在每次启动 `cargo test`，后台应用程序将运行在一个随机端口上。但是运行 `cargo test` 测试失败了，HTTP 客户端仍然在调用 `127.0.0.1:8000`。

我们需要找出操作系统赋予我们的应用程序的端口，有以下几种方法可以做到，我们将使用 [`std::net::TcpListener`](https://doc.rust-lang.org/beta/std/net/struct.TcpListener.html)。我们 **`HttpServer`** 现在的逻辑是，获取一个地址参数，绑定该地址，然后启动应用程序，我们可以接管第一步：自己用 **`TcpListener`** 绑定端口，然后使用 [`listen`](https://docs.rs/actix-web/4.0.1/actix_web/struct.HttpServer.html#method.listen)。

有什么好处？

[`TcpListener::local_addr`](https://doc.rust-lang.org/beta/std/net/struct.TcpListener.html#method.local_addr) 返回一个 [`SocketAddr`](https://doc.rust-lang.org/beta/std/net/enum.SocketAddr.html) 它公开了我们绑定的实际端口 [`.port()`](https://doc.rust-lang.org/beta/std/net/enum.SocketAddr.html#method.port)

先修改 `run` 函数

```rust
//! src/lib.rs
use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpResponse, HttpServer};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
        .listen(listener)?
        .run();
    Ok(server)
}
```

修改 `main.rs`

```rust
//! src/main.rs

use std::net::TcpListener;

use zero2prod::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let address = TcpListener::bind("127.0.0.1:8000")?;
    run(address)?.await
}
```

修改 `spawn_app` 函数

```rust
//! tests/health_check.rs

use std::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();
    let client = reqwest::Client::new();
    // Act
    let response = client
        .get(&format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    // We return the application address to the caller!
    format!("http://127.0.0.1:{}", port)
}
```

运行 `cargo test` 一切正常，现在程序更加的健壮了。

## 完成第一个用户故事

> As a blog visitor, I want to subscribe to the newsletter, So that I can receive email updates when new content is published on the blog.

我们希望博客访问者在网页上的表单输入他们的邮件地址，该表单将除法对我们后端 `API` 的 **POST** `/subscriptions` 调用，该 `API` 将实际处理信息，存储信息并返回响应。

- 在 `actix-web` 中读取请求的数据
- Rust 中用于连接 `PostgreSQL` 的第三方库
- 如何设置数据库和管理迁移
- 如何在 `API` 请求处理程序中获得数据库连接
- 如何在我们集成测试中存储数据
- 当使用数据库时，如何避免测试之间操作交互

## 使用 HTML 表单

### 细化需求

需要订阅者的称呼和邮件地址，通过 HTML 表单手机，将在 POST 请求的 body 中传递给后端 API。body 中的数据如何 encoded，有很多方法，可以使用 HTML 表单的 `application/x-www-form-urlencoded` 方法。

引用 [MDN web docs](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/POST)

> the keys and values [in our form] are encoded in key-value tuples separated by ‘&’, with a ‘=’ between the key and the value. Non-alphanumeric characters in both keys and values are percent encoded.

例如，用户名是  Le Guin 邮箱是 ursula_le_guin@gmail.com，POST 请求体 body 应该是

```shell
name=le%20guin&email=ursula_le_guin%40gmail.com
```

此时空格被转换成了 `%20`，`@` 被转换成了 `%40`，可以在[这里](https://www.w3schools.com/tags/ref_urlencode.ASP)找到对应的转换

总结一下：

- 如果使用 `application/x-www-form-urlencoded` 提供了有效的姓名和电子邮箱，后端应该返回一个 200 OK
- 如果姓名或地址为空，后端应该返回一个 400 错误请求

### 将需求写成测试

有了具体的需求，先写测试明确我们的期望

```rust
//! tests/health_check.rs

use std::net::TcpListener;

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    // We return the application address to the caller!
    format!("http://127.0.0.1:{}", port)
}

// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)
#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();
    // Act
    let response = client
        .get(&format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_from_data() {
    // Arrange
    let app_address = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app_address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/subscriptions", &app_address))
            .header("Content-type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
```

**subscribe_returns_a_400_when_data_is_missing** 是一个表格驱动测试 [table-driven test](https://github.com/golang/go/wiki/TableDrivenTests) 或称为参数化测试的一个示例。这在处理错误输入时特别有用，可以简单地对一组已知错误进行相同的断言，而不是多次重复测试逻辑。

对于参数化测试，在失败时有好的错误消息是很重要的，需要知道是哪个特定的输入无效，另一方面，参数化覆盖了很多领域，因此多花一点时间来生成一个好的失败消息是有意义的。

### 解析 POST 请求中的表单数据

在 `src/lib.rs` 中添加对应的路由

```rust
//! src/lib.rs
use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpResponse, HttpServer};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn subscribe() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
    })
    .listen(listener)?
    .run();
  
    Ok(server)
}
```

#### Extractors

在 [`actix-web 用户指南`](https://actix.rs/docs/) 中 [`Extractors`](https://actix.rs/docs/extractors/) 提取器是非常重要的部分。

`actix-web` 提供了几个开箱即用的提取器，以满足最常见的使用场景：

- [Path](https://actix.rs/docs/extractors/) 从请求的路径中获取动态路径参数
- [Query](https://docs.rs/actix-web/4.0.1/actix_web/web/struct.Query.html) 用于查询参数
- [Json](https://docs.rs/actix-web/4.0.1/actix_web/web/struct.Json.html) 解析 JSON 编码的请求体
- [From](https://docs.rs/actix-web/4.0.1/actix_web/web/struct.Form.html) 解析表单的条件
- 等等

我们这里使用 From 提取器，之前阅读文档

> Form data helper (application/x-www-form-urlencoded).
>
> Can be used to extract url-encoded data from the request body, or send url-encoded data as the response.

如何使用，查看 [actix-web 用户指导](https://actix.rs/docs/extractors/)

> An extractor can be accessed as an argument to a handler function. Actix-web supports up to 10 extractors per handler function. Argument position does not matter.

URL-Encoded Forms Example

```rust
use actix_web::{post, web, App, HttpServer, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct FormData {
    username: String,
}

/// extract form data using serde
/// this handler gets called only if the content type is *x-www-form-urlencoded*
/// and the content of the request could be deserialized to a `FormData` struct
#[post("/")]
async fn index(form: web::Form<FormData>) -> Result<String> {
    Ok(format!("Welcome {}!", form.username))
}
```

按照上面的示例修改 `src/lib.rs` 中的 `subscribe` 处理函数

```rust
//! src/lib.rs
// [...]

#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
```

 在 `Cargo.toml` 中添加依赖 `serde`

```toml
[dependencies]
# We need the optional `derive` feature to use `serde`'s procedural macros:
# `#[derive(Serialize)]` and `#[derive(Deserialize)]`.
# The feature is not enabled by default to avoid pulling in
# unnecessary dependencies for projects that do not need it.
serde = {version = "1", features = ["derive"]}
```

此时运行 `cargo test` 测试应该已经通过了

```shell
running 3 tests
test health_check_works ... ok
test subscribe_returns_a_200_for_valid_from_data ... ok
test subscribe_returns_a_400_when_data_is_missing ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

#### Form 和 FromRequest

查看 [`Form`](https://github.com/actix/actix-web/blob/be986d96b387f9a040904a6385e9500a4eb5bb8f/actix-web/src/types/form.rs) 的源码

```rust
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Form<T>(pub T);
```

这个定义非常简单，`Form` 是泛型 `T` 的一个包装，是用于填充表单的唯一字段。

`Form` 可以作为提取器类型，是因为它实现了 [`FromRequest`](https://docs.rs/actix-web/4.0.1/actix_web/trait.FromRequest.html) `trait`。

```rust
pub trait FromRequest: Sized {
    type Error: Into<Error>;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future;

    fn extract(req: &HttpRequest) -> Self::Future { ... }
}
```

因为目前 Rust 还不支持 trait 定义中包含 async fn，稍微修改以下，上面的定义可以简化为如下内容

```rust
/// Trait implemented by types that can be extracted from request.
///
/// Types that implement this trait can be used with `Route` handlers.
pub trait FromRequest: Sized {
    type Error = Into<actix_web::Error>;
  
    async fn from_request(
        req: &HttpRequest,
        payload: &mut Payload
    ) -> Result<Self, Self::Error>;
  
    /// Omitting some ancillary methods that actix-web implements
    /// out of the box for you and supporting associated types
    /// [...]
}
```

`from_request` 函数的入参是 **HTTP** 请求的 [`HttpRequest`](https://docs.rs/actix-web/4.0.1/actix_web/struct.HttpRequest.html) 和 [`Payload`](https://docs.rs/actix-web/4.0.1/actix_web/dev/enum.Payload.html)，如果提取成功返回 `Self`，失败则转换为 [`actix_web::Errror`](https://docs.rs/actix-web/4.0.1/actix_web/struct.Error.html) 的错误类型。

路由处理函数签名中的所有参数都必须实现 **FromRequest** `trait`，**actix-web** 将为每个参数调用 [`from_request`](https://github.com/actix/actix-web/blob/01cbef700fd9d7ce20f44bed06c649f6b238b9bb/src/handler.rs#L212) 函数，如果所有参数都提取成功，则运行实际的处理函数。如果其中一个提取失败，则返回相应的错误给调用者，并且永远不会执行处理程序（**actix_web::Error** 可以转换成 **HttpResponse**）。

这非常高效的处理了输入参数，可以直接使用强类型信息，从而大大简化了处理请求所需编写的代码。

让我们看下 [**Form** 的 **FromRequest**](https://github.com/actix/actix-web/blob/01cbef700fd9d7ce20f44bed06c649f6b238b9bb/src/types/form.rs#L112) 到底做了什么？

```rust
impl<T> FromRequest for Form<T>
where
    T: DeserializeOwned + 'static,
{
    type Config = FormConfig;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req2 = req.clone();
        let (limit, err) = req
            .app_data::<FormConfig>()
            .map(|c| (c.limit, c.ehandler.clone()))
            .unwrap_or((16384, None));

        UrlEncoded::new(req, payload)
            .limit(limit)
            .map(move |res| match res {
                Err(e) => {
                    if let Some(err) = err {
                        Err((*err)(e, &req2))
                    } else {
                        Err(e.into())
                    }
                }
                Ok(item) => Ok(Form(item)),
            })
            .boxed_local()
    }
}
```

这里稍微改变下实际代码，突出关键元素，忽略实现细节

```rust
impl<T> FromRequest for Form<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    async fn from_request(req: &HttpRequest, payload: &mut Payload) -> Result<Self, Self::Error> {
        // Omitted stuff around extractor configuration (e.g. payload size limits)
        match UrlEncoded::new(req, payload).await {
            Ok(item) => Ok(Form(item)),
            // The error handler can be customised.
            // The default one will return a 400, which is what we want.
            Err(e) => Err(error_handler(e)),
        }
    }
}
```

主要内容发生在 `UrlEncoded` 结构体内部，UrlEncoded 做了很多事情，它透明地处理压缩和未压缩地有效载荷，它以字节流地形式处理请求体等等。

这[主要代码](https://github.com/actix/actix-web/blob/01cbef700fd9d7ce20f44bed06c649f6b238b9bb/src/types/form.rs#L358)处理完成之后，就是

```rust
serde_urlencoded::from_bytes::<T>(&body).map_err(|_| UrlencodedError::Parse)
```

[serde_urlencoded](https://docs.rs/serde_urlencoded/0.6.1/serde_urlencoded/) 为 `application/x-www-form-urlencoded` 格式的数据提供序列化支持。

**from_bytes** 接受一个 &[u8] ，并根据 URL 编码格式的规范从其中去序列化一个 T 类型的实例。

因为 T 从 **serde** 实现了 **DeserializedOwned** trait

```rust
impl<T> FromRequest for Form<T>
where
    T: DeserializeOwned + 'static,
{
    // [...]
}
```

#### Rust 中的序列化

如果要了解实际情况，可以仔细阅读 [**serde** 文档](https://serde.rs/)。我们为什么需要 **serde**，**serde** 实际上为我们做了什么？

> Serde is a framework for ***ser\***ializing and ***de\***serializing Rust data structures efficiently and generically.

#### Putting Everything Together 综上所述

回顾之前所学的一切，让我们看看订阅的处理程序

```rust
#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}
// Let's start simple: we always return a 200 OK
async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
```

我们现在对这段代码有一个很好的理解了

- 在调用 **subscribe** 之前，**actix-web** 为 **subscribe** 的所有输入参数调用 **from_request** 方法，在我们的例子中是 **Form::from_request**。
- **Form::from_request** 尝试根据 **URL 编码规则**将请求体 **body** 反序列化为 **FormData** 结构体，利用 **serde_urlencoded** 和 **FormData** 的 **Deserialize trait** 实现。由 **#[derive(serde::deserize)]** 自动为我们生成。
- 如果 **Form::from_request** 失败，则向调用者返回一个 400 错误请求，如果成功了 **subscribe** 被调用，返回 200 OK。

代码如此简单，但其中却发生了太多的事情 -- 这非常依赖 **Rust** 的类型系统和它的生态系统中的一些 **crates**。

## 存储数据：数据库

目前我们的 `POST /subscriptions` 接口通过了我们的测试，但是它的作用非常有限，因为我们没有在任何地方存储有效的电子邮件和姓名。

使用 sqlx 作为数据库

### 集成测试

之前的测试没有判断订阅的用户信息是否已经被正确保存到数据库了。

```rust
//! tests/health_check.rs
// [...]

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_from_data() {
    // Arrange
    let app_address = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());
}
```

我们有两个选择：

1. 利用另外的公开 API 进行检查数据库是否正确保存了数据
2. 在我们的测试用例中，直接查询数据库

如果可能的话，选项1应该是首选的，因为你的测试应该不关心 API 的实现细节（例如底层数据库技术），这样不太可能被未来的重构所干扰。但是，我们的 API 上没有任何公共的接口来验证订阅的用户是否存在。

我们可以添加一个 **GET /subscriptions** 接口来获取现有的订阅用户列表，但是我们必须担心如何保护它，我们不希望在没有任何形式的身份验证的情况下，将我们的订阅用户姓名和电子邮件暴露在公共互联网上。

我们可能最终会编写一个 **GET /subscriptions** 接口，也就是说，我们不应该仅仅是为了测试我们正在开发的功能而开始编写新的功能。

现在可以在测试中编写一个查询，当更好的测试策略可用时，可以重构它。

### Database Setup

#### Docker

使用 [postgres 数据库官方的 docker 镜像](https://hub.docker.com/_/postgres)来启动数据库，可以按照 [docker 官网指导](https://hub.docker.com/_/postgres)来在本机安装 docker，让我们先创建一个小的脚本 `scripts/init_db.sh` 来定制 Postgres 的默认设置

```shell
#!/usr/bin/env bash
set -x
set -eo pipefail
# Check if a custom user has been set, otherwise default to 'postgres'
DB_USER=${POSTGRES_USER:=postgres}
# Check if a custom password has been set, otherwise default to 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
# Check if a custom database name has been set, otherwise default to 'newsletter'
DB_NAME="${POSTGRES_DB:=newsletter}"
# Check if a custom port has been set, otherwise default to '5432'
DB_PORT="${POSTGRES_PORT:=5432}"
# Launch postgres using Docker
docker run \
    -e POSTGRES_USER=${DB_USER} \
    -e POSTGRES_PASSWORD=${DB_PASSWORD} \
    -e POSTGRES_DB=${DB_NAME} \
    -p "${DB_PORT}":5432 \
    -d postgres \
    postgres -N 1000
    # ^ Increased maximum number of connections for testing purposes
```

变成可执行文件

```bash
chmod +x scripts/init_db.sh
```

我们可以执行这个脚本来启动 Postgres

```bash
./scripts/init_db.sh
```

运行 `docker ps` 可以看到下面这行的数据库信息

```bash
IMAGE PORTS STATUS
postgres 127.0.0.1:5432->5432/tcp Up 12 seconds [...]
```

#### Database Migrations

为了存储我们的订阅用户信息，我们需要创建第一张表。要向我们的数据添加一个新表，我们需要改变它的 [schema](https://www.postgresql.org/docs/9.5/ddl-schemas.html) - 这通常被称为数据库迁移。

**sqlx** 提供了一个命令行交互工具 [`sql-cli`]() 来管理数据库迁移。我们可以使用下面命令安装

```bash
cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres
```

安装成功后，运行 `sqlx --help` 查看是否正常工作

**Database Creation**

一般来说第一个命令是创建数据库，查看文档 `sqlx database create`

```bash
> sqlx database create --help

sqlx.exe-database-create 
Creates the database specified in your DATABASE_URL

USAGE:
    sqlx.exe database create --database-url <DATABASE_URL>

OPTIONS:
    -D, --database-url <DATABASE_URL>
            Location of the DB, by default will be read from the DATABASE_URL env var [env:
            DATABASE_URL=]

    -h, --help
            Print help information
```

但是对于我们来说不是必要的，因为在 Postgres Docker 脚本中已经启动了一个名为 newsletter 的默认数据库。尽管如此，我们还是须在在 CI 和生成环境中经历创建步骤。正如文档提示，`sqlx database create` 依赖于 `DATABASE_URL` 环境变量。`DATABASE_URL` 应该是有效的 Postgres 连接字符串，格式如下：

```bash
postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
```

因此，我们需要在 `scripts/init_db.sh` 中添加几行代码

```shell
# [...]
export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
```

你有时可能会遇到一个问题，当我们试图运行 `sql database create` 时，Postgres 容器将无法接收连接。

我们需要在开始运行命令之前，等待 Postgres 容器已经正常启动，在运行状态，所以将脚本更新为：

```shell
#!/usr/bin/env bash
set -x
set -eo pipefail
DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_PORT="${POSTGRES_PORT:=5432}"
docker run \
    -e POSTGRES_USER=${DB_USER} \
    -e POSTGRES_PASSWORD=${DB_PASSWORD} \
    -e POSTGRES_DB=${DB_NAME} \
    -p "${DB_PORT}":5432 \
    -d postgres \
    postgres -N 1000

# Keep pinging Postgres until it's ready to accept commands
export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
    echo >&2 "Postgres is still unavailable - sleeping"
    sleep 1
done

echo >&2 "Postgres is up and running on port ${DB_PORT}!"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
```

健康检查使用了 Postgres 的命令行客户端  psql，如果在本机先[安装](https://www.timescale.com/blog/how-to-install-psql-on-mac-ubuntu-debian-windows/)，如果没有安装 psql 和 sqlx-cli，运行脚本会报错，我们来修改下脚本

```sh
#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed."
    echo >&2 "Use:"
    echo >&2 " cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi
# [...]
```

**Adding A Migration**

让我们创建第一个迁移，在命令行运行以下命令

```bash
# Assuming you used the default parameters to launch Postgres in Docker!
export DATABASE_URL=postgres://postgres:password@127.0.0.1:5432/newsletter
sqlx migrate add create_subscriptions_table
```

现在一个新的顶级目录 `migrations` 出现在你的项目中，sqlx 的 CLI 将在这里存储我们项目的所有迁移。

在 `migrations` 目录下，应该已经有了一个名为 `{timestamp}_create_subscriptions_table.sql` 的文件。

让我们在这里写下第一个 SQL 语句

```sql
-- Add migration script here
-- migrations/{timestamp}_create_subscriptions_table.sql
-- Create Subscriptions Table
CREATE TABLE subscriptions(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    subscribed_at timestamptz NOT NULL
);
```

当提到数据库表的主键（[primary keys](https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-PRIMARY-KEYS)）就会有[无休止的争论](https://www.mssqltips.com/sqlservertip/5431/surrogate-key-vs-natural-key-differences-and-when-to-use-in-sql-server/)，有些人希望使用具有业务意义的列（比如：电子邮件，*natural key*），有些人觉得使用没有任何业务意义的合成键更加安全（例如：id、随机生成的 UUID、*surrogate key*）。通常默认使用合成标识符。

这里还需要注意几件事情：

- [timestamptz](https://www.postgresqltutorial.com/postgresql-timestamp/) 是时区感知的日期和时间类型
- 我们在数据库级别使用[唯一约束](https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-UNIQUE-CONSTRAINTS)
- 我们强制要求所有字段都应该填充一个非空约束](https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-UNIQUE-CONSTRAINTS)在每列上
- 我们正在使用 [TEXT](https://www.postgresql.org/docs/current/datatype-character.html)，因为我么对它的最大长度没有任何限制

数据库约束时抵御应用程序错误的最后一道防线，但这是有代价的 - 在将新数据写入到表之前，数据库必须确保所有检查都通过。因此，约束会影响我们的写吞吐量，即单位时间内我们可以在表中**插入/更新**的行数。

特别是，**UNIQUE** 在我们的点至邮件列中引入了一个额外的 B树索引，该索引必须在每次**插入/更新/删除**查询时进行更新，并且会占用磁盘空间。

**Running Migrations**

我们可以使用以下工具针对我们的数据库运行迁移

```bash
sqlx migrate run
```

它具有于 `sqlx database create` 相同的行为，它将查看 `DATABASE_URL` 环境变量，以了解需要迁移什么数据库。让我们将它添加到脚本 `scripts/init_db.sh` 中

```sh
#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed."
    echo >&2 "Use:"
    echo >&2 " cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi

DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_PORT="${POSTGRES_PORT:=5432}"

# Allow to skip Docker if a dockerized Postgres database is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
    docker run \
        -e POSTGRES_USER=${DB_USER} \
        -e POSTGRES_PASSWORD=${DB_PASSWORD} \
        -e POSTGRES_DB=${DB_NAME} \
        -p "${DB_PORT}":5432 \
        -d postgres \
        postgres -N 1000
fi

# Keep pinging Postgres until it's ready to accept commands
export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT} - running migrations now!"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"
```

我们将 `docker run` 命令放在 `SKIP_DOCKER` 标志之后，这样就可以轻松地对现有的 Postgres 实例运行迁移，而不必手动将其关闭，并使用 `scripts/init_db.rs` 重新创建它。如果 Postgres 不是由我们的脚本启动的，它在 CI 中也很有用。

现在我们可以用以下方法迁移数据库

```bash
SKIP_DOCKER=true ./scripts/init_db.sh
```

你应该可以在终端输出中发现类似这样的内容

```bash
+ sqlx migrate run
Applied 20220414172840/migrate create subscriptions table (80.1576ms)
```

如果使用 **Postgres** 的[图形化界面](https://www.pgadmin.org/)查看，可以看到除了 **subscriptions** 表之外还有一个新的 **_sqlx_migrations** 表，这是 **sqlx** 跟踪对数据库运行了哪些迁移的地方，对于我们的 **create_subscriptions_table** 迁移，它现在应该包含一个单独的行。

### 写第一个查询语句

#### 配置 `sqlx` 依赖

首先在 `Cargo.toml` 中添加 `sqlx` 的依赖

```rust
#! Cargo.toml
[dependencies]
# [...]

# Using table-like toml syntax to avoid a super-long line!
[dependencies.sqlx]
default-features = false
features = [
  "runtime-actix-rustls",
  "macros",
  "postgres",
  "uuid",
  "chrono",
  "migrate",
]
version = "0.5"
```

这里面有很多特征标志：

- `runtime-actix-rustls` 告诉 sqlx 将 actix 运行时用于 future，并将 rustls 作为 TLS 后端
- `macros` 让我们可以使用 `sqlx::query!` 和 `sqlx::query_as!`，这两个宏我们将广泛使用
- `postgres` 将开启 Postgres 的功能（例如：非标准的 SQL 类型）
- `uuid`  使用 [`uuid crate`](https://docs.rs/uuid/) 添加对 SQL UUIDs 映射到 Uuid 类型。我们需要私用它处理 id 列
- `chrone` 使用 [`chrono crate`](https://docs.rs/chrono/) 支持将 SQL 的 timestamptz 转换成 `DateTime<T>` 类型
- `migrate` 使我们能够访问 `sqlx-cli` 管理迁移时使用的功能，将应用于集成测试

#### 配置数据库管理

连接 Postgres 数据库最简单的方式是 [`PgConnection`](https://docs.rs/sqlx/0.5.1/sqlx/struct.PgConnection.html)。`PgConnection` 实现了 [`Connection trait`](https://docs.rs/sqlx/0.5.1/sqlx/prelude/trait.Connection.html) 提供的 [`connect method`](https://docs.rs/sqlx/0.5.1/sqlx/prelude/trait.Connection.html#method.connect) 。它接受一个连接字符串为输入，并异步返回一个 **Result<PostgresConnection, sqlx::Error>**。

从哪里获取连接字符串？可以使用硬编码，也可以选择开始引入一个基本的配置文件。选择使用 [`config crate`](https://docs.rs/config/)

#### 重构目录



