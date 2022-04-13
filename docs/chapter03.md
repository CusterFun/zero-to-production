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
          token: ${{ secrets.GITHUB name: Security audit
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
          token: ${{ secrets.GITHUB_TOKEN }}_TOKEN }}
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
          args: '--ignore-tests'
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

根据设计 Rust 标准库中不包括异步运行时，你可以将任何一个异步运行时作为依赖引入到项目中，以满足实际的特定需求（如 [Fuchsia project](http://smallcultfollowing.com/babysteps/blog/2019/12/09/async-interview-2-cramertj/#async-interview-2-cramertj) 或者 [bastion's actor](https://github.com/bastion-rs/bastion) 框架）。

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









