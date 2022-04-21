- [1.  Unkonwn Unkonwns](#1--unkonwn-unkonwns)
- [2. 可观测性](#2-可观测性)
- [3. Logging 日志记录](#3-logging-日志记录)
  - [3.1 The `log` Crate](#31-the-log-crate)
  - [3.2 `actix-web` 的 `Logger Middleware`](#32-actix-web-的-logger-middleware)
  - [3.3 The Facade Patter](#33-the-facade-patter)
- [4. Instrumenting POST /subscriptions](#4-instrumenting-post-subscriptions)
  - [4.1  Interactions With External Systems](#41--interactions-with-external-systems)
  - [4.2 Think Like A User](#42-think-like-a-user)
  - [4.3 Logs Must Be Easy To Correlate](#43-logs-must-be-easy-to-correlate)
- [5. 结构化日志](#5-结构化日志)
  - [5.1 `tracing crate`](#51-tracing-crate)
  - [5.2 Migrating From log To tracing](#52-migrating-from-log-to-tracing)
  - [5.3 tracing’s Span](#53-tracings-span)
  - [5.4 Instrumenting Futures](#54-instrumenting-futures)
  - [5.5  tracing’s Subscriber](#55--tracings-subscriber)
  - [5.6 tracing-subscriber](#56-tracing-subscriber)
  - [5.7 tracing-bunyan-formatter](#57-tracing-bunyan-formatter)
  - [5.8 tracing-log](#58-tracing-log)
  - [5.9 Removing Unused Dependencies](#59-removing-unused-dependencies)
  - [5.10  Cleaning Up Initialisation](#510--cleaning-up-initialisation)
  - [5.11 Logs For Integration Tests](#511-logs-for-integration-tests)
  - [5.12 Cleaning Up Instrumentation Code - tracing::instrument](#512-cleaning-up-instrumentation-code---tracinginstrument)
  - [5.13  Protect Your Secrets - secrecy](#513--protect-your-secrets---secrecy)
  - [5.14  Request Id](#514--request-id)
  - [5.15    Leveraging The tracing Ecosystem](#515----leveraging-the-tracing-ecosystem)
- [6. 总结](#6-总结)

在第3章中，我们完成了 `PUT /subscriptions`，以实现项目的第一个用户故事：

> As a blog visitor, 
> I want to subscribe to the newsletter, 
> So that I can receive email updates when new content is published on the blog.

我们还没有创建一个带有 HTML 表单的 web 页面来实际测试端到端流程，但是我们有一些黑盒集成测试，涵盖了我们在这个阶段关心的两个基本场景

- 如果提交了有效的数据（即提供了姓名和电子邮件），数据将保存到我们的数据库中
- 如果提交的表单不完整（例如：缺少电子邮件或姓名），API 将返回 400

现在我们已经准备好部署我们的应用程序到生成环境了吗？

并没有，我们的应用程序还没有搜集日志，这使得我们容易受到未知的影响，不清楚具体发生的问题。

## 1.  Unkonwn Unkonwns

我们的测试是正确的，但不能保证我们的程序时正确的，我们可能会遇到一些我们没有测试过的场景，甚至一开始设计应用程序时就没有考虑过。所以我们需要一些[形式化的方法](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf)，比如

- 如果我们失去了与数据库的连接会怎么样？`sqlx::PgPool` 是否会尝试自动恢复，或者从那时起知道我们重新启动应用程序之前，所有数据库交互都将失败？
- 如果攻击者试图在表单中提交恶意的数据，会发生什么？（比如：非常大的数据，或尝试 [SQL 注入攻击](https://en.wikipedia.org/wiki/SQL_injection)等）

如果有足够的时间和努力，我们可以摆脱许多已知的未知，但是有些问题是以前没有出现过的，无法预料，这就是未知的未知。有时经验足以将未知的未知转化为已知的未知：如果你以前从未使用过数据库，可能不会意识到当失去数据库连接时会发生什么，一旦你遇到过这种情况一次，它就会称为一种常见的故障。

通常情况下，未知的未知是我们正在研究的特定系统特有的故障模式。它们是处于我们的软件组件、底层操作系统，我们正在使用的硬件、我们开发过程中的问题等等。它可能会在下列情况中出现：

- 系统被推到其他正常操作条件之外（例如：一个寻常的高峰流量）
- 多个组件同时出现故障（例如：当数据库正在进行[主副本故障切换](https://www.postgresql.org/docs/current/warm-standby-failover.html)时，一个 SQL 事务被挂起）
- 引入了改变系统平衡的变化（例如：调整了失败重试的策略）
- 很长时间没有引入任何变化（例如：应用程序已经几周没有重新启动，可能会看到各种各样的内存泄露）
- 等等

所有这些场景都有一个关键的相似之处：它们通常不可能在生成环境之外重现。我们能做些什么准备来应对未知原因引用的中断或错误呢？

## 2. 可观测性

当一个未知问题出现时，我们可能在做其他事情，没有在出错的第一时间注意到，将调试器连接到生成环境正在运行的进程上，（这往往也是不现实的，不可能的），并且性能下降可能会同时影响多个系统。我们唯一可以依靠的是**日志数据**，来了解和调试这个未知的问题，关于我们正在运行的应用程序的信息，这些信息是自动收集的，以后可以通过检查来回答关于系统在某一时间点的状态的问题。

什么问题？如果它是一个未知的问题，我们并不真正事先知道我们可能需要问什么问题来判断出它的根本原因 -- 这就是问题的关键。所以要有一个**可观察的应用程序**，引述自 [Honeycomb的可观察性指南](https://www.honeycomb.io/what-is-observability/)

> Observability is about being able to ask arbitrary questions about your environment without — and this is the key part — having to know ahead of time what you wanted to ask.
>
> 可观察性是指能够对你的环境提出任意的问题
> 这就是关键部分--必须提前知道你想问什么。

简而言之，要建立一个可观测的系统，我们需要

- 对我们的应用程序进行检测，以收集高质量的日志数据。 
- 使用工具和系统，以便对数据进行切分、切割和处理，为我们的问题找到答案。

## 3. Logging 日志记录

日志是最常见的可观察性数据类型。当遇到问题时，你会查看日 志，以期望了解发生了什么，希望能捕获足够的信息来有效地进行故障排除。

不过，什么是日志？ 其格式各不相同，取决于你所使用的时代、平台和技术。现在的日志记录通常是一堆文本数据，用 换行符把当前记录和下一个记录分开。

```
The application is starting on port 8080
Handling a request to /index
Handling a request to /index
Returned a 200 OK
```

上面是四条完全有效的日志记录。 当涉及到记录时，Rust生态系统能为我们提供什么？

### 3.1 The [`log` Crate](https://docs.rs/log)

Rust 的 `log` 提供了五个宏：[`trace`](https://docs.rs/log/0.4.11/log/macro.trace.html), [`debug`](https://docs.rs/log/0.4.11/log/macro.debug.html), [`info`](https://docs.rs/log/0.4.11/log/macro.info.html), [`warn`](https://docs.rs/log/0.4.11/log/macro.warn.html) 和 [`error`](https://docs.rs/log/0.4.11/log/macro.error.html)。

`trace` 是最低级别的：`trace` 级别的日志通常非常冗长，而且信噪比很低（例如，每次网络服务器收到 一个TCP数据包时都会发出一条 `trace` 级别的日志记录）。 然后，我们按照严重程度递增的顺序，有调试、信息、警告和错误。 错误级日志用于报告可能对用户产生影响的严重故障（例如，我们未能处理一个传入的请求或对数 据库的查询超时）。

让我们快速看一个例子

```rust
fn fallible_operation() -> Result<String, String> {}
pub fn main() {
    match fallible_operation() {
        Ok(success) => {
            log::info!("Operation succeeded: {}", success);
        }
        Err(err) => {
            log::error!("Operation failed: {}", err);
        }
    }
}
```

我们正试图执行一个可能失败的操作。如果它成功了 ，我们会发出一条信息级的日志记录。 如果失败，我们会发出一条错误级别的日志记录。

### 3.2 `actix-web` 的 `Logger Middleware`

`actix-web` 提供了日志中间件 [`Logger Middleware`](https://docs.rs/actix-web/latest/actix_web/middleware/struct.Logger.html)，它为每个传入的请求发出一条日志记录。让我们把它 添加到我们的应用程序中。

```rust
//! src/startup.rs
use actix_web::dev::Server;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

use crate::routes::{health_check, subscribe};

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            // Middlewares are added using the `wrap` method on `App`
            .wrap(Logger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
```

现在我们可以使用 `cargo run` 启动应用程序，并使用 

```sh
curl http://127.0.0.1:8000/health_check -v
```

快速发送请求。 请求的结果是200，但是在我们用来启动应用程序的终端上什么也没有发生。 没有日志。什么都没有。空白的屏幕。

### 3.3 The Facade Patter

应用程序不知道该如何处理这些日志记录？ 应该把它们添加到一个文件中？还是应该把它们打印到终端？还是通过HTTP将它们发送到一个远程系统（例如 [ElasticSearch](https://www.elastic.co/elasticsearch/)）？

Rust 内置的 log 利用 [`facade pattern`](https://en.wikipedia.org/wiki/Facade_pattern) 来处理这种问题？

它给你提供了你所需要的工具来发日志记录，但它并没有规定这些日志记录应该如何处理。相反 ，它提供了一个[`Log trait`](https://docs.rs/log/latest/log/trait.Log.html)。

```rust
//! From `log`'s source code - src/lib.rs
/// A trait encapsulating the operations required of a logger.
pub trait Log: Sync + Send {
    /// Determines if a log message with the specified metadata would be logged.
    ///
    /// This is used by the `log_enabled!` macro to allow callers to avoid
    /// expensive computation of log message arguments if the message would be
    /// discarded anyway.
    fn enabled(&self, metadata: &Metadata) -> bool;
    /// Logs the `Record`.
    ///
    /// Note that `enabled` is *not* necessarily called before this method.
    /// Implementations of `log` should perform all necessary filtering
    /// internally.
    fn log(&self, record: &Record);
    /// Flushes any buffered records.
    fn flush(&self);
}
```

在主函数的开始，你可以调用 [`set_logger`] (https://docs.rs/log/latest/log/fn.set_logger.html) 函数，并传递一个Log特性的实现：每当有日志记录发出时，**Log::log** 就会在你提供的日志器上被调用，因此可以对你认为必要的日志记录进行任何形式的处理。 如果你不调用 `set_logger`，那么所有的日志记录将被简单地丢弃。

[`crates.io`](https://crates.io/) 上有一些Log的实现，这里我们将使用 [`env_logger`](https://docs.rs/env_logger)，如果像我们的情况一样，主要目标是将所有的日志记录打印到终端，那么它的效果很好。 `cargo add env_logger`

```toml
#! Cargo.toml
# [...]
[dependencies]
env_logger = "0.9"
```

`env_logger::Logger` 使用以下格式将日志记录打印到终端。

```
[<timestamp> <level> <module path>] <log message>
```

`env_logger::Logger` 查看 **RUST_LOG** 环境变量来决定哪些日志应该被打印，哪些日志应该被过滤掉。 例如，**RUST_LOG=debug cargo run**，将显示所有由我们的应用程序或我们正在使用的 `crate` 发出的调试级别或更高的日志。而 **RUST_LOG=zero2prod**，则会过滤掉所有由我们依赖 `crate` 所发出的日志。 让我们按照要求修改我们的 `main.rs` 文件。

```rust
//! src/main.rs
use env_logger::Env;
use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // `init` does call `set_logger`, so this is all we need to do.
    // We are falling back to printing all logs at info-level or above
    // if the RUST_LOG environment variable has not been set.
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Panic if we can't read configuratio
    let configuration = get_configuration().expect("Failed to get configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address, connection_pool)?.await
}
```

现在运行 `cargo run` 相当于（ `RUST_LOG=info cargo run`，由于 `Env::default().default_filter_or("info")`）, 如果我们用 `curl http://127.0.0.1:8000/health_check ` 发出请求，你应该看到日志记录，由我们添加的 `Logger` 中间件发出。 日志也是探索我们正在使用的软件如何工作的一个很棒的工具。试着将设置 `RUST_LOG=trace`，并再次启动该应用程序。 你应该看到一堆注册的轮询日志记录，这些记录来自于mio，一个用于非阻塞性IO的底层库，以及 `actix-web` 的日志。 通过 `trace-level` 级别的日志，可以了解到一些底层的逻辑。

## 4. Instrumenting POST /subscriptions

让我们用上面关于日志的知识来处理 `POST /subscriptions` 请求。它目前看起来像这样。

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // `Result` has two variants: `Ok` and `Err`.
    // The first for successes, the second for failures.
    // We use a `match` statement to choose what to do based on the outcome.
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            println!("Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

让我们把 `log` 作为一个依赖加入 `Cargo.toml`。

```toml
#! Cargo.toml
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
config = {version = "0.13", default-features = false, features = ["yaml"]}
tokio = {version = "1", features = ["macros", "rt-multi-thread"]}
# We need the optional `derive` feature to use `serde`'s procedural macros:
# `#[derive(Serialize)]` and `#[derive(Deserialize)]`.
# The feature is not enabled by default to avoid pulling in
# unnecessary dependencies for projects that do not need it.
chrono = "0.4"
env_logger = "0.9"
log = "0.4"
serde = {version = "1", features = ["derive"]}
uuid = {version = "0.8", features = ["v4"]}

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

[dev-dependencies]
reqwest = "0.11"

```

我们应该在日志记录中记录什么？

### 4.1  Interactions With External Systems

让我们从一个久经考验的经验法则开始：任何通过网络与外部系统的互动都应该被密切监控。我们可能会遇到网络问题，数据库可能无法使用，随着时间的推移，查询可能会变得更慢，因为订阅用户列表会变长，等等。 让我们添加两条日志记录：一条在查询执行开始前，一条在查询完成后立即执行。 查询成功时发出一条日志记录。为了捕捉失败，我们需要将查询失败时的 `println` 语句转换成 `error-level` 错误级别的日志。

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    log::info!("Saving new subscriber details in the database");
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            log::info!("New subscriber details have");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            log::error!("Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

好了--我们现在在某种程度上解决了这个问题。 

注意一个细节：我们使用 `{:?}`，即 [`std::fmt::Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) 格式，来捕获查询错误。 开发人员是日志的主要受众--我们应该尽可能多地提取有关发生的任何故障的信息，以方便故障排除。 `Debug` 给我们提供了这种原始视图，而 [`std::fmt::Display ({})`](https://doc.rust-lang.org/std/fmt/trait.Display.html) 将返回一个更漂亮的错误信息，更适合直接显示给我们的终端用户。

### 4.2 Think Like A User

我们还应该记录什么？设身处地为你的用户着想，一个登陆你网站的人对你发布的内容感兴趣，并想订阅你的通讯。 对他们来说，失败是什么样子的？

> Hey! 
> I tried subscribing to your newsletter using my main email address, thomas_mann@hotmail.com, but the website failed with a weird error. Any chance you could look into what happened?

汤姆登陆了我们的网站，当他按下提交按钮时收到了 "一个奇怪的错误"。

如果我们能从他提供给我们的信息线索（即他输入的电子邮件地址）中找出问题，我们的应用可以说是*可观察的*，我们能做到吗？

首先，让我们确认一下这个问题：汤姆是否注册为用户？ 我们可以连接到数据库，并运行一个快速查询，以仔细检查是否有以下记录 thomas_mann@hotmail.com 作为电子邮件在我们的订阅者表中。就算数据库中存在该邮件地址，但是我们的日志不包括用户的电子邮件地址，所以我们无法搜索到它。我们可以要求汤姆提供额外的信息：我们所有的日志记录都有一个时间戳，也许如果他记得他试图订阅 的大约时间，我们可以挖出一些东西。

这清楚地表明，我们目前的日志还不够好。让我们来改进它们。

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // We are using the same interpolation syntax of `println`/`print` here!
    log::info!(
        "Adding '{}' '{}' as a new subscriber.",
        form.email,
        form.name
    );
    log::info!("Saving new subscriber details in the database");
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            log::info!("New subscriber details have");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            log::error!("Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

我们现在新增一个记录，同时保存姓名和电子邮件。

### 4.3 Logs Must Be Easy To Correlate

这足以解决 汤姆的问题了吗？

如果我们的网络服务器在任何时候都只有一个副本在运行，而且这个副本一次只能处理一个请求，我们可以想象日志显示在我们的终端，或多或少是这样。

```shell
# First request
[.. INFO zero2prod] Adding 'thomas_mann@hotmail.com' 'Tom' as a new subscriber
[.. INFO zero2prod] Saving new subscriber details in the database
[.. INFO zero2prod] New subscriber details have been saved
[.. INFO actix_web] .. "POST /subscriptions HTTP/1.1" 200 ..
# Second request
[.. INFO zero2prod] Adding 's_erikson@malazan.io' 'Steven' as a new subscriber
[.. ERROR zero2prod] Failed to execute query: connection error with the database
[.. ERROR actix_web] .. "POST /subscriptions HTTP/1.1" 500 ..
```

你可以清楚地看到一个请求从哪里开始，在我们试图处理它的时候发生了什么，我们返回的响应是什么 ，下一个请求从哪里开始，等等。 它很容易遵循。 但当你同时处理多个请求时，情况就不是这样了。

```shell
[.. INFO zero2prod] Receiving request for POST /subscriptions
[.. INFO zero2prod] Receiving request for POST /subscriptions
[.. INFO zero2prod] Adding 'thomas_mann@hotmail.com' 'Tom' as a new subscriber
[.. INFO zero2prod] Adding 's_erikson@malazan.io' 'Steven' as a new subscriber
[.. INFO zero2prod] Saving new subscriber details in the database
[.. ERROR zero2prod] Failed to execute query: connection error with the database
[.. ERROR actix_web] .. "POST /subscriptions HTTP/1.1" 500 ..
[.. INFO zero2prod] Saving new subscriber details in the database
[.. INFO zero2prod] New subscriber details have been saved
[.. INFO actix_web] .. "POST /subscriptions HTTP/1.1" 200 ..
```

但我们没能保存什么细节？thomas_mann@hotmail.com 还是s_erikson@malazan.io？不可能从日志中看出来。 我们需要一种方法来关联与同一请求相关的所有日志。 这通常是通过一个请求ID（**request id** 也被称为**correlation id**）来实现的：当我们开始处理一个传入的请求时，我们会生成一个随机的标识符（例如[UUID](https://en.wikipedia.org/wiki/Universally_unique_identifier)），然后将其与所有关于完成该特定请求的日志相关联。 让我们给我们的处理程序添加一下。

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // Let's generate a random unique identifier
    let request_id = Uuid::new_v4();
    log::info!(
        "request_id {} - Adding '{}' '{}' as a new subscriber.",
        request_id,
        form.email,
        form.name
    );
    log::info!("request_id {request_id} - Saving new subscriber details in the database");
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            log::info!("request_id {request_id} - New subscriber details have been saved");

            HttpResponse::Ok().finish()
        }
        Err(e) => {
            log::error!("request_id {request_id} - Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

现在可以发送一个请求

```shell
curl -i -X POST -d 'email=thomas_mann@hotmail.com&name=Tom' \
http://127.0.0.1:8000/subscriptions
```

查看日志

```shell
[2022-04-18T08:18:35Z INFO  actix_server::builder] Starting 4 workers
[2022-04-18T08:18:35Z INFO  actix_server::server] Tokio runtime found; starting in existing Tokio runtime
[2022-04-18T09:07:00Z INFO  zero2prod::routes::subscriptions] request_id a33c47e0-e4e0-47dc-a776-a73be26dedc9 - Adding 'thomas_mann@hotmail.com' 'Tom' as a new subscriber.
[2022-04-18T09:07:00Z INFO  zero2prod::routes::subscriptions] request_id a33c47e0-e4e0-47dc-a776-a73be26dedc9 - Saving new subscriber details in the database
[2022-04-18T09:07:00Z INFO  sqlx::query] INSERT INTO subscriptions (id, …; rows affected: 1, rows returned: 0, elapsed: 48.067ms
    
    INSERT INTO
      subscriptions (id, email, name, subscribed_at)
    VALUES
      ($1, $2, $3, $4)

[2022-04-18T09:07:00Z INFO  zero2prod::routes::subscriptions] request_id a33c47e0-e4e0-47dc-a776-a73be26dedc9 - New subscriber details have been saved
[2022-04-18T09:07:00Z INFO  actix_web::middleware::logger] 127.0.0.1 "POST /subscriptions HTTP/1.1" 200 0 "-" "curl/7.80.0" 0.278990
[2022-04-18T09:07:00Z INFO  sqlx::query] /* SQLx ping */; rows affected: 0, rows returned: 0, elapsed: 474.000µs
```

一个传入请求的日志现在看起来像这样。 我们现在可以在我们的日志中搜索thomas_mann@hotmail.com，找到第一条记录，记住 `request_id`，然后找到与该请求相关的所有其他日志记录。 `request_id` 是在我们的订阅处理程序中创建的，因此 `actix_web` 的 `Logger` 中间件完全不知道它。 这意味着，我们将不知道当用户试图订阅我们的服务时，我们的应用程序向他们返回了什么状态代码。 我们应该怎么做？ 我们可以删除 `actix_web` 的 `Logger`，写一个中间件来为每个传入的请求生成一个随机的请求标识符，然后写我们自己的日志中间件，知道这个标识符并将其包含在所有日志行中。 它能发挥作用吗？是的。 我们应该这样做吗？也许不应该。

## 5. 结构化日志

为了确保 **request_id** 包含在所有的日志记录中，我们将不得不。 •

- 重写请求处理管道中的所有上游组件（例如 **actix-web** 的 **Logger** ）。 
-  改变我们从 **subscribe** 处理程序中调用的所有下游函数的签名；如果它们需要打日志，那么它们需要包括**request_id**，因此需要将其作为一个参数传递下来。

那么，由我们使用的第三方 **crate** 所发出的日志记录呢？我们是否也应该重写这些记录？ 很明显，这种方法是不现实的。

让我们退一步：我们的代码看起来像什么？ 

我们有一个总体任务（一个HTTP请求），它被分解成一组子任务（例如解析输入，进行查询等），这些子任务又可能被递归地分解成更小的子程序。

- 每一个工作单元都有一个*持续时间*（即开始和 结束）。 
- 这些工作单位中的每一个都有一个与之相关的 *context*（例如，新用户的姓名和电子邮件。 **request_id**），自然被其所有工作的子单元所共享。 

毫无疑问，我们正在挣扎：日志语句是发生在一个确定的时间点上的孤立事件，我们顽固地试图用 它来表示一个树状的处理管道。 日志是一个**错误的抽象概念**。 那我们应该用什么呢？

### 5.1 [`tracing crate`](https://docs.rs/tracing) 

> **tracing** expands upon logging-style diagnostics by allowing libraries and applications to record structured events with additional information about temporality and causality — unlike a log message, a span in tracing has a beginning and end time, may be entered and exited by the flow of execution, and may exist within a nested tree of similar spans.

> 追踪是对日志式诊断的扩展，它允许库和应用程序记录带有时间性和因果关系的额外信息的结构化事件--与日志信息不同，追踪中的跨度有一个开始和结束时间，可以通过执行流程进入和退出，并可能存在于类似跨度的嵌套树中。

这对我们来说是件好事。 它在实践中是什么样子的？

### 5.2 Migrating From log To tracing

让我们把我们的订阅处理程序 `subscribe` 转换为使用 **tracing** 而非 **log** 来进行记录。先添加 `tracing` 到我们的依赖

```toml
#! Cargo.toml
#[...]

[dependencies]
tracing = {version = "0.1", features = ["log"]}

#[...]
```

迁移第一步：搜索所有出现的在我们的函数主体中的日志字符串并替换成 **tracing**

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // Let's generate a random unique identifier
    let request_id = Uuid::new_v4();
    tracing::info!(
        "request_id {} - Adding '{}' '{}' as a new subscriber.",
        request_id,
        form.email,
        form.name
    );
    tracing::info!("request_id {request_id} - Saving new subscriber details in the database");
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            tracing::info!("request_id {request_id} - New subscriber details have been saved");

            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("request_id {request_id} - Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

重新运行 `cargo run` 再发送一个 `POST /subscriptions` 请求，可以再控制台看到完全相同的日志

```shell
curl -i -X POST -d 'email=thomas_mann@hotmail.com&name=Tom' \
http://127.0.0.1:8000/subscriptions
```

这要归功于我们在 **Cargo.toml** 中启用的 [tracing’s log feature flag](https://docs.rs/tracing/latest/tracing/index.html#log-compatibility)。它确保每次使用 **tracing’s macros** 创建事件或跨度时，都会发出相应的日志事件，使 **log** 日志的记录器能够接收到它（在我们的例子中，是 **env_logger**）。

### 5.3 tracing’s Span 

我们现在可以开始利用 **tracing’s [Span](https://docs.rs/tracing/latest/tracing/span/index.html) ** 来更好地捕捉我们程序的结构。我们想创建一个 **span** 来代表整个 HTTP 请求的跨度。

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // Let's generate a random unique identifier
    let request_id = Uuid::new_v4();
    // Spans, like logs, have an associated level
    // `info_span` creates a span at the info-level
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email=%form.email,
        subriber_name=%form.name
    );
    // Using `enter` in an async function is a recipe for disaster!
    // Bear with me for now, but don't do this at home.
    // See the following section on `Instrumenting Futures`
    let _request_span_guard = request_span.enter();

    tracing::info!(
        "request_id {} - Adding '{}' '{}' as a new subscriber.",
        request_id,
        form.email,
        form.name
    );
    tracing::info!("request_id {request_id} - Saving new subscriber details in the database");
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            tracing::info!("request_id {request_id} - New subscriber details have been saved");

            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("request_id {request_id} - Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }

    // `_request_span_guard` is dropped at the end of `subscribe`
    // That's when we "exit" the span
}
```

修改的内容

```rust
/! src/routes/subscriptions.rs
// [...]

pub async fn subscribe(/* */) -> HttpResponse {
    let request_id = Uuid::new_v4();
    // Spans, like logs, have an associated level
    // `info_span` creates a span at the info-level
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name= %form.name
    );
    // Using `enter` in an async function is a recipe for disaster!
    // Bear with me for now, but don't do this at home.
    // See the following section on `Instrumenting Futures`
    let _request_span_guard = request_span.enter();
    
    // [...]
    // `_request_span_guard` is dropped at the end of `subscribe`
    // That's when we "exit" the span
}
```

这里有很多事情要做--让我们把它分解以下。

 我们正在使用 `info_span!` 宏来创建一个新的 **span**，并为其上下文 **context** 附加一些值: `request_id`、`form.email` 和 `form.name`。 
我们不再使用字符串插值：**tracing** 允许我们将结构化信息作为键值对的集合与我们的 **span** 相关联。
我们可以显示的命名它们（例如，`subscriber_email` 用于 `form.email`），
或者隐式地使用变量名作为键 （例如：`request_id` 相当于 `request_id = request_id`）。
请注意，我们在它们的前缀上了**%符号**：我们告诉 **tracing** 使用它们的 **Display实现**来做日志。
你可以在他们的[文档](https://docs.rs/tracing/latest/tracing/#recording-fields)中找到更多关于其他可用选项的细节。 

**info_span** 返回新创建的 **span**，但我们必须使用 **.enter()** 方法来激活它。

**.enter()** 返回 [`Entered`](https://docs.rs/tracing/latest/tracing/span/struct.Entered.html) 的实例，这是一个守护变量 *guard*：
只要守护变量没有被丢弃，所有的下游跨度和日志事件都将被注册为 **span children**。
这是一个典型的Rust模式，通常被称为资源获取即初始化（ [`RAII`](https://doc.rust-lang.org/stable/rust-by-example/scope/raii.html)）：编译器会跟踪所有变量的生命周期，当它们超出范围时，会插入对其析构器的调用，即 [`Drop::drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html)。 

**Drop trait** 的默认实现只负责释放该变量所拥有的资源。 不过，我们可以指定一个自定义的 **Drop** 实现来执行其他清理 操作--例如，当 [**Entered** guard 被丢弃时从 **span** 中退出](https://docs.rs/tracing/latest/tracing/span/struct.Entered.html#impl-Drop)。

```rust
//! `tracing`'s source code
impl<'a> Drop for Entered<'a> {
    #[inline]
    fn drop(&mut self) {
        // Dropping the guard exits the span.
        //
        // Running this behaviour on drop rather than with an explicit function
        // call means that spans may still be exited when unwinding.
        if let Some(inner) = self.span.inner.as_ref() {
            inner.subscriber.exit(&inner.id);
        }
        if_log_enabled! {{
            if let Some(ref meta) = self.span.meta {
                self.span.log(
                    ACTIVITY_LOG_TARGET,
                    log::Level::Trace,
                    format_args!("<- {}", meta.name())
                );
            }
        }}
    }
}
```

检查你的依赖关系的源代码往往可以发现一些特别的东西--我们刚刚发现，如果启用了 **log feature flag** 日志功能标志，当 **span** 退出时，**tracing** 将发出 **trace-level** 日志。 让我们立即行动起来吧。

```shell
 RUST_LOG=trace cargo run
```

运行 curl

```shell
curl -i -X POST -d 'email=thomas_mann@hotmail.com&name=Tom' http://127.0.0.1:8000/subscriptions
```

可以看到终端日志

```shell
[.. INFO zero2prod] Adding a new subscriber.; request_id=f349b0fe..
subscriber_email=ursulale_guin@gmail.com subscriber_name=le guin
[.. TRACE zero2prod] -> Adding a new subscriber.
[.. INFO zero2prod] request_id f349b0fe.. - Saving new subscriber details
in the database
[.. INFO zero2prod] request_id f349b0fe.. - New subscriber details have
been saved
[.. TRACE zero2prod] <- Adding a new subscriber.
[.. TRACE zero2prod] -- Adding a new subscriber.
[.. INFO actix_web] .. "POST /subscriptions HTTP/1.1" 200 ..
```

注意我们在 **span** 的上下文中捕获的所有信息是如何在发射的日志行中报告的。
我们可以通过发射的日志密切关注我们的跨度的生命周期。 

- 添加一个新的订阅者在创建跨度时被记录下来。 
- 我们进入跨度（->）。 
- 我们执行 **INSERT** 查询。 
- 我们退出跨度（<-）。 
- 我们最终关闭了这个跨度（--）。 

退出和关闭跨度之间有什么区别？很高兴你问了! 你可以多次进入（和退出）一个跨度。而关闭则是最终的：它发生在跨度本身被放弃的时候。 当你有一个可以暂停然后恢复的工作单元时，这就非常方便了--比如说一个异步任务。

### 5.4 Instrumenting Futures

让我们用数据库查询作为一个例子。 **executor** 必须不断地[轮询](https://doc.rust-lang.org/beta/std/future/trait.Future.html#tymethod.poll)来查看 **future** 是否已经完成。-- 当该 **future** 处于闲置状态时，我们可以在其他 **future** 上取得进展。

 这显然会引起问题：我们如何确保不把它们各自的跨度**span**混在一起？ 最好的方法是密切模仿 **future** 的生命周期：我们应该在每次执行器轮询时进入与我们的 **future** 相关的跨度， 并在它被停放时退出。 

这就是 [`Instrument`](https://docs.rs/tracing/latest/tracing/trait.Instrument.html) 的作用。它是 **futures** 的一个扩展 **trait**。**Instrument::instrument** 做的正是我们想要的： 每次自我轮询 **self**，即 **future**，都会进入我们作为参数传递的跨度；每次 **future** 结束，都会退出这个跨度。

 让我们在我们的查询上试一试。 

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use tracing::Instrument;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // Let's generate a random unique identifier
    let request_id = Uuid::new_v4();
    // Spans, like logs, have an associated level
    // `info_span` creates a span at the info-level
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email=%form.email,
        subriber_name=%form.name
    );
    // Using `enter` in an async function is a recipe for disaster!
    // Bear with me for now, but don't do this at home.
    // See the following section on `Instrumenting Futures`
    let _request_span_guard = request_span.enter();
    // We do not call `.enter` on query_span!
    // `.instrument` takes care of it at the right moments
    // in the query future lifetime
    let query_span = tracing::info_span!("Saving new subscriber details in the database");

    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    // First we attach the instrumentation, then we `.await` it
    .instrument(query_span)
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            // Yes, this error log falls outside of `query_span`
            // We'll rectify it later, pinky swear!
            tracing::error!("Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }

    // `_request_span_guard` is dropped at the end of `subscribe`
    // That's when we "exit" the span
}
```

如果我们在 `RUST_LOG=trace` 的情况下再次启动应用程序，

```rust
//! src/main.rs
//! [...]
#[tokio::main]
async fn main() -> std::io::Result<()> {
	env_logger::from_env(Env::default().default_filter_or("trace")).init();
	// [...]
}
```

并尝试 `POST /subscriptions` 请求，我们 将看到与这些看起来有些相似的日志

```shell
curl -i -X POST -d 'email=thomas_mann@hotmail.com&name=Tom' \
http://127.0.0.1:8000/subscriptions
```

```bash
[.. INFO zero2prod] Adding a new subscriber.; request_id=f349b0fe..
subscriber_email=ursulale_guin@gmail.com subscriber_name=le guin
[.. TRACE zero2prod] -> Adding a new subscriber.
[.. INFO zero2prod] Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] -> Saving new subscriber details in the database
[.. TRACE zero2prod] <- Saving new subscriber details in the database
[.. TRACE zero2prod] -- Saving new subscriber details in the database
[.. TRACE zero2prod] <- Adding a new subscriber.
[.. TRACE zero2prod] -- Adding a new subscriber.
[.. INFO actix_web] .. "POST /subscriptions HTTP/1.1" 200 ..
```

我们可以清楚地看到，在完成之前，查询的 `future` 被 `exector` 轮询查询了多少次。

### 5.5  tracing’s Subscriber

我们开始了从 **log** 到 **tracing** 的迁移，因为我们需要一个更好的抽象概念来有效地检测我们的代码。我们特别想把**request_id** 附加到所有与同一个传入的 **HTTP** 请求相关的日志中。 虽然我保证 **tracing** 会解决我们的问题，但看看这些日志：**request_id** 只在第一条日志语句中被打印出 来，但是我们已经把它明确地附加到 **span**上下文中。 这是为什么呢？ 

嗯，我们还没有完成我们的迁移。 

尽管我们把所有的 **instrumentation code** 从 **log** 转移到了 **tracing**，但我们仍然在使用 **env_logger** 来处理一切! 

```rust
//! src/main.rs
//! [...]
#[tokio::main]
async fn main() -> std::io::Result<()> {
	env_logger::from_env(Env::default().default_filter_or("info")).init();
	// [...]
}
```

**env_logger** 的 **logger** 实现了 **log** 的 **Log trait**--它对 **tracing** 的 **Span** 所暴露的丰富结构一无所知! 

**tracing** 与 **log** 的兼容性是很好的开始，但现在是时候用 **tracing-native** 解决方案来取代 **env_logger** 。 

**tracing crate** 遵循 **log** 所使用的 **facade pattern** 模式--你可以自由地使用它的宏来检测你的代码，但应用程序负责说明该 **span** **telemetry** 数据应该如何被处理。 

[`Subscriber`](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html) is the tracing counterpart of log’s Log:：`Subscriber trait` 的实现暴露了各种方法来管理 **Span** 生命周期的每个阶段--创建、进入/退出、关闭等。 

```rust
//! `tracing`'s source code
pub trait Subscriber: 'static {
    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id;
    fn event(&self, event: &Event<'_>);
    fn enter(&self, span: &span::Id);
    fn exit(&self, span: &span::Id);
    fn clone_span(&self, id: &span::Id) -> span::Id;
    // [...]
}
```

**tracing** 的文档质量令人叹为观止--我强烈邀请你自己看一看 [`Subscriber`的文档](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html)，以正确理解这些方法的每一个作用。

### 5.6 tracing-subscriber

追踪并不是开箱即用的。

我们需要使用 [`tracing-subscriber`](https://docs.rs/tracing-subscriber)，这是在 `tracing` 项目上维护的另一个 **crate**，以找到一些基本的订阅者来启动（to find a few basic subscribers to get off the ground.）。让我们把它添加到我们的依赖项中。 

```toml
#! Cargo.toml
#[...]
[dependencies]
tracing-subscriber = {version="0.3", features = ["registry","env-filter"]}
#[...]
```

**tracing-subscriber** 的作用远不止是为我们提供几个方便的 **subscriber**。它将另一个关键 **trait** 引入进来，即 [`Layer`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html)。 **Layer** 使我们有可能为 **span** 数据建立一个处理管道：我们并不被迫提供一个包罗万象的 **subscriber**，做我们想要 的一切；相反，我们可以结合多个较小的层来获得我们需要的处理管道。

 这大大减少了整个 **tracing** 生态系统中的重复工作：人们专注于通过生产新的层来增加新的能力，而不是试图建立一个*best-possible-batteries-included subscriber*。 

layer 方法的基础是 [`Registry`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/struct.Registry.html)。 

**Registry** 实现了 **Subscriber trait**，并负责所有相关的事务。

> Registry does not actually record traces itself: instead, it collects and stores span data that is exposed to any layer wrapping it […]. The Registry is responsible for storing span metadata, recording relationships between spans, and tracking which spans are active and which are closed.
>
> Registry本身并不实际记录 trace：相反，它收集并存储暴露在任何层包裹中的跨度数据[...]。Registry负责存储跨度元数据，记录跨度之间的关系，并跟踪哪些跨度是活动的，哪些是关闭的。

下游层可以利用 **Registry** 的功能，专注于他们的目的：过滤应该处理哪些 **span**，格式化跨度数据， 将跨度数据运送到远程系统，等等。

### 5.7 tracing-bunyan-formatter

我们想把 **subscriber** 放在一起，它的功能与 **env_logger** 相同。我们将通过结合三层 **Layer** 来达到这个目的。 --  --

- [`tracing_subscriber::filter::EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/struct.EnvFilter.html) 根据它们的日志级别和它们的来源来丢弃 **span**，就像我们在 **env_logger** 中通过 **RUST_LOG** 环境变量所做的那样。
- [`tracing_bunyan_formatter::JsonStorageLayer`](https://docs.rs/tracing-bunyan-formatter/latest/tracing_bunyan_formatter/struct.JsonStorageLayer.html) 处理 **span** 数据，并将相关的元数据以易于消费的 **JSON** 格式存储给下游层。特别是，它可以将上下文从父跨度传播到它们的子跨度
- [`tracing_bunyan_formatter::BunyanFormatterLayer`](https://docs.rs/tracing-bunyan-formatter/latest/tracing_bunyan_formatter/struct.BunyanFormattingLayer.html) 构建在 **JsonStorageLayer** 之上。 并以与 [`bunyan`](https://docs.rs/tracing-bunyan-formatter/latest/tracing_bunyan_formatter/struct.BunyanFormattingLayer.html) 兼容的**JSON** 格式输出日志记录。

让我们把 [`tracing_bunyan_formatter`](https://docs.rs/tracing-bunyan-formatter/latest/tracing_bunyan_formatter/) 添加到我们的依赖项。

```toml
#! Cargo.toml
#[...]
tracing-subscriber = {version="0.3", features = ["registry","env-filter"]}
tracing-bunyan-formatter = "0.3"
#[...]
```

现在我们可以在我们的主函数中把一切都联系起来。 

```rust
//! src/main.rs
use env_logger::Env;
use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // `init` does call `set_logger`, so this is all we need to do.
    // We are falling back to printing all logs at info-level or above
    // if the RUST_LOG environment variable has not been set.
    env_logger::Builder::from_env(Env::default().default_filter_or("trace")).init();

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to get configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address, connection_pool)?.await
}
```

修改代码之后

```rust
//! src/main.rs
use std::net::TcpListener;

use sqlx::PgPool;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // We removed the `env_logger` line we had before!

    // We are falling back to printing all logs at info-level or above
    // if the RUST_LOG environment variable has not been set.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(),
        // Output the formatted spans to stdout.
        std::io::stdout,
    );
    // The `with` method is provided by `SubscriberExt`, an extension
    // trait for `Subscriber` exposed by `tracing_subscriber`
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    // `set_global_default` can be used by applications to specify
    // what subscriber should be used to process spans.
    set_global_default(subscriber).expect("Failed to set subscriber");

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to get configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address, connection_pool)?.await
}
```

运行 `cargo run`，再使用 `curl` 发送请求 `curl -i -X POST -d 'email=thomas_mann@hotmail.com&name=Tom'  http://127.0.0.1:8000/subscriptions` ，可以看到终端日志形式如下:

```shell
{
    "msg": "[ADDING A NEW SUBSCRIBER - START]",
    "subscriber_name": "le guin",
    "request_id": "30f8cce1-f587-4104-92f2-5448e1cc21f6",
    "subscriber_email": "ursula_le_guin@gmail.com"
    ...
}
{
    "msg": "[SAVING NEW SUBSCRIBER DETAILS IN THE DATABASE - START]",
    "subscriber_name": "le guin",
    "request_id": "30f8cce1-f587-4104-92f2-5448e1cc21f6",
    "subscriber_email": "ursula_le_guin@gmail.com"
    ...
}
{
    "msg": "[SAVING NEW SUBSCRIBER DETAILS IN THE DATABASE - END]",
    "elapsed_milliseconds": 4,
    "subscriber_name": "le guin",
    "request_id": "30f8cce1-f587-4104-92f2-5448e1cc21f6",
    "subscriber_email": "ursula_le_guin@gmail.com"
    ...
}
{
    "msg": "[ADDING A NEW SUBSCRIBER - END]",
    "elapsed_milliseconds": 5
    "subscriber_name": "le guin",
    "request_id": "30f8cce1-f587-4104-92f2-5448e1cc21f6",
    "subscriber_email": "ursula_le_guin@gmail.com",
    ...
}
```

我们成功的将附加到原始上下文的所有内容都已经传播到了它的所有子跨度。
`Tracing-bunyan-formatter` 还提供了开箱即用的 `duration`：每当一个跨度被关闭，一条JSON消息就会被打印到控制台，并附加一个 `elapsed_millisecond` 属性。
 当涉及到搜索时，JSON格式是非常友好的：一个像 **ElasticSearch** 这样的引擎可以我们可以轻松地搜索所有这些记录，推断出一个模式，并对 `request_id`、`name` 和 `email` 字段进行索引 。它释放了查询引擎的全部力量，以筛选需要的日志! 这比我们以前的情况要好得多：为了进行复杂的搜索，我们不得不使用自定义的重码，因此大大限制 了我们可以轻易向日志提出问题的范围。

### 5.8 tracing-log

如果你仔细看看，你会发现我们在这一过程中丢失了一些东西：我们的终端只显示由我们的应用程序直接发出的日志。`actix-web` 的日志记录发生了什么？ 
`tracing log feature flag` 确保每次 `traceing` 事件发生时都会发出一条日志记录，让 `log` 的记录器来接收它们。
反之则不然：`log` 并不是一开始就发出 `tracing` 事件，也没有提供一个功能标志来启用这种行为。 
如果我们想要的话，我们需要明确地注册一个日志记录器的实现，将日志重定向到我们的 `tracing` 系统中去办理的用户。 
我们可以使用由 [`tracing-log`](https://docs.rs/tracing-log) crate提供的 [`LogTracer`](https://docs.rs/tracing-log/latest/tracing_log/struct.LogTracer.html)。添加到 `Cargo.toml`

```toml
#! Cargo.toml
# [...]
[dependencies]
tracing-log = "0.1"
# [...]
```

让我们根据需要修改我们的 `main.rs`

```rust
//! src/main.rs
//! [...]
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_log::LogTracer;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Redirect all `log`'s events to our subscriber
    LogTracer::init().expect("Failed to set logger");

    let env_filter = EnvFilter::try_from_default_env()
    	.unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(),
        std::io::stdout
    );
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set subscriber");
    
    // [...]
}
```

所有 `actix-web` 的日志应该再次出现在我们的控制台。

### 5.9 Removing Unused Dependencies

删除未使用的依赖关系，如果你快速扫描一下我们所有的文件，你会发现我们没有使用 `log` 或 `env_logger`，我们应该把它们从 `Cargo.toml` 文件中删除。

在一个大项目中，要发现一个依赖在重构后变得未被使用是非常困难的。幸运的是，工具再一次拯救了我们--让我们安装 `cargo-udeps`（未使用的依赖项）。

```shell
cargo install cargo-udeps
```

`cargo-udeps` 扫描你的 `Cargo.toml` 文件，并检查 `[dependencies]` 下列出的所有装箱是否真的在项目中使用过。
请查看 [`cargo-deps' trophy case`](https://github.com/est31/cargo-udeps#trophy-case)，其中有一长串流行的 `Rust` 项目，`cargo-udeps` 能够发现未使用的依赖，并缩``短构建时间。 让我们在我们的项目上运行它!

```bash
# cargo-udeps requires the nightly compiler.
# We add +nightly to our cargo invocation
# to tell cargo explicitly what toolchain we want to use.
cargo +nightly udeps
```

输出的是，

```shell
unused dependencies:
`zero2prod v0.1.0 (D:\Up\zero2prod)`
└─── dependencies
     └─── "env_logger"
```

它并没有找到 `env_logger` 的使用。 让我们从 `Cargo.toml` 文件中删除。

### 5.10  Cleaning Up Initialisation

我们不遗余力地向前推进，以改善我们应用程序的可观察性。 现在让我们退一步，看看我们写的代码，看看我们是否能以任何有意义的方式进行改进。 让我们从我们的主函数开始。

```rust
//! src/main.rs
use std::net::TcpListener;

use sqlx::PgPool;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Redirect all `log`'s events to our subscriber
    LogTracer::init().expect("Failed to set logger");

    // We are falling back to printing all logs at info-level or above
    // if the RUST_LOG environment variable has not been set.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(),
        // Output the formatted spans to stdout.
        std::io::stdout,
    );
    // The `with` method is provided by `SubscriberExt`, an extension
    // trait for `Subscriber` exposed by `tracing_subscriber`
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    // `set_global_default` can be used by applications to specify
    // what subscriber should be used to process spans.
    set_global_default(subscriber).expect("Failed to set subscriber");

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to get configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address, connection_pool)?.await
}
```

现在这个主函数里有很多事情要做，让我们把它分解一下。

```rust
//! src/main.rs
use std::net::TcpListener;

use sqlx::PgPool;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use zero2prod::{configuration::get_configuration, startup::run};

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to
/// spell out the actual type of the returned subscriber, which is
/// indeed quite complex.
/// We need to explicitly call out that the returned subscriber is
/// `Send` and `Sync` to make it possible to pass it to `init_subscriber`
/// later on.
pub fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting_layer = BunyanFormattingLayer::new(name, std::io::stdout);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}
/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into());
    init_subscriber(subscriber);

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to get configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address, connection_pool)?.await
}
```

我们现在可以把 `get_subscriber` 和 `init_subscriber` 移到我们的 `zero2prod` `library` 中的一个模块 `telemetry`。

```rust
//! src/lib.rs
pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;
```

```rust
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to
/// spell out the actual type of the returned subscriber, which is
/// indeed quite complex.
/// We need to explicitly call out that the returned subscriber is
/// `Send` and `Sync` to make it possible to pass it to `init_subscriber`
/// later on.
pub fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting_layer = BunyanFormattingLayer::new(name, std::io::stdout);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
```

```rust
//! src/main.rs
use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into());
    init_subscriber(subscriber);

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to get configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address, connection_pool)?.await
}
```

### 5.11 Logs For Integration Tests

我们不仅仅是为了审美/可读性的原因而进行清理--我们要把这两个函数移到 `zero2prod library`中，使它们对我们的测试也可用。 
作为一个经验法则，我们在应用程序中使用的一切都应该反映在我们的集成测试中。特别是结构化的日志，可以大大加快我们在集成测试失败时的调试速度：我们可能不需要附加调试器，更多的时候，日志可以告诉我们哪里出了问题。这也是一个很好的基准：如果你不能从日志中进行调试，想象一下在生产中进行调试会是多么的困难啊。

让我们改变我们的 `spawn_app` 辅助函数来处理初始化我们的 `tracing stack`：

```rust
//! tests/health_check.rs

use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

// The function is asynchronous now!
async fn spawn_app() -> TestApp {
    let subscriber = get_subscriber("test".into(), "debug".into());
    init_subscriber(subscriber);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}
// [...]
```

此时运行 `cargo test`，可以看到一长串的测试失败

```shell
failures:
---- subscribe_returns_a_400_when_data_is_missing stdout ----
thread 'subscribe_returns_a_400_when_data_is_missing' panicked at
'Failed to set logger: SetLoggerError(())'
Panic in Arbiter thread.

---- subscribe_returns_a_200_for_valid_form_data stdout ----
thread 'subscribe_returns_a_200_for_valid_form_data' panicked at
'Failed to set logger: SetLoggerError(())'
Panic in Arbiter thread.

failures:
    subscribe_returns_a_200_for_valid_form_data
    subscribe_returns_a_400_when_data_is_missing
```

`init_subscriber` 应该只被调用一次，但是它被我们所有的测试调用了。
我们可以使用 **once_cell** 来纠正它：

```toml
#! Cargo.toml
# [...]
[dev-dependencies]
once_cell = "1"
# [...]
```

```rust
//! tests/health_check.rs

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber("test".into(), "debug".into());
    init_subscriber(subscriber);
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

// The function is asynchronous now!
async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}
// [...]
```

现在运行 `cargo test` 一切正常。

不过，输出是非常嘈杂的：我们有几条日志线从每个测试案例中出来。 我们希望我们的跟踪工具在每个测试中都得到测试，但我们不希望每次运行测试套件时都要看这些日志。 `cargo test` 解决了 **println/print** 语句的同样问题。默认情况下，它将所有打印到控制台的东西都忽略了。你可以明确选择查看那些打印语句，使用 

```bash
cargo test -- --nocapture
```

我们需要为我们的 **tracing** 制定一个同等的策略。 让我们为 **get_subscriber** 添加一个新的参数，以允许自定义日志应该写到哪里。

```rust
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to
/// spell out the actual type of the returned subscriber, which is
/// indeed quite complex.
/// We need to explicitly call out that the returned subscriber is
/// `Send` and `Sync` to make it possible to pass it to `init_subscriber`
/// later on.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    // This "weird" syntax is a higher-ranked trait bound (HRTB)
    // It basically means that Sink implements the `MakeWriter`
    // trait for all choices of the lifetime parameter `'a`
    // Check out https://doc.rust-lang.org/nomicon/hrtb.html
    // for more details.
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
```

 然后我们需要调整我们的主函数以使用 `stdout`。

```rust
//! src/main.rs
use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to get configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address, connection_pool)?.await
}
```

在我们的测试代码中，我们将根据一个环境变量 `TEST_LOG` 动态地选择输出的地方。如果 `TEST_LOG` 被设置， 我们就使用 `std::io::stdout`。 如果没有设置 `TEST_LOG`，我们就用 `std::io::sink` 把所有的日志都送入`void`。 我们自己的自制版本 `--nocapture` 标志。

```rust
//! tests/health_check.rs

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the value of `TEST_LOG`
    // because the sink is part of the type returned by `get_subscriber`, therefore they are not the
    // same type. We could work around it, but this is the most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

// [...]
```

当你想看到某个测试案例的所有日志来调试它时，

```shell
# We are using the `bunyan` CLI to prettify the outputted logs
# The original `bunyan` requires NPM, but you can install a Rust-port with
# `cargo install bunyan`
TEST_LOG=true cargo test health_check_works | bunyan
```

你可以运行并对输出进行筛选，以了解正在发生的事情。

### 5.12 Cleaning Up Instrumentation Code - tracing::instrument

我们重构了我们的初始化逻辑。现在让我们来看看我们的 **instrumentation** 代码。

再来修改 `subscribe` 函数

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // Let's generate a random unique identifier
    let request_id = Uuid::new_v4();
    // Spans, like logs, have an associated level
    // `info_span` creates a span at the info-level
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email=%form.email,
        subriber_name=%form.name
    );
    // Using `enter` in an async function is a recipe for disaster!
    // Bear with me for now, but don't do this at home.
    // See the following section on `Instrumenting Futures`
    let _request_span_guard = request_span.enter();
    // We do not call `.enter` on query_span!
    // `.instrument` takes care of it at the right moments
    // in the query future lifetime
    let query_span = tracing::info_span!("Saving new subscriber details in the database");

    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    // First we attach the instrumentation, then we `.await` it
    .instrument(query_span)
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            // Yes, this error log falls outside of `query_span`
            // We'll rectify it later, pinky swear!
            tracing::error!("Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }

    // `_request_span_guard` is dropped at the end of `subscribe`
    // That's when we "exit" the span
}
```

可以说，`log` 给我们的 `subscribe` 功能增加了一些噪音。让我们看看我们是否能把它减少一点。 
我们将从 **request_span** 开始：我们希望 **subscribe** 中的所有操作都发生在 **request_span** 的上下文中。 
换句话说，我们想把 **subscribe** 函数包裹在一个 **span** 中。 
这个要求相当普遍：在自己的函数中提取每个子任务是结构化例程的常见方式，以提高可读性，使其更容易编写测试；因此，我们经常希望在函数声明中附加一个 **span** 。 
**tracing** 用它的 **tracing::instrument procedural macro** 来满足这个特殊的使用情况。让我们看看它的作用。

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let query_span = tracing::info_span!("Saving new subscriber details in the database");

    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    // First we attach the instrumentation, then we `.await` it
    .instrument(query_span)
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            // Yes, this error log falls outside of `query_span`
            // We'll rectify it later, pinky swear!
            tracing::error!("Failed to execute query {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }

    // `_request_span_guard` is dropped at the end of `subscribe`
    // That's when we "exit" the span
}
```

`#[tracing::instrument]` 在函数调用开始时创建一个 **span**，并自动将传递给函数的所有参数附加到 **span** 的上下文中--在我们的例子中，**form** 和 **pool**。通常情况下，函数参数不会在日志记录中显示（比如**pool**），或者我们想更明确地指定应该/如何捕获它们（比如命名 **form** 的每个字段）--我们可以明确地告诉 **tracing** 使用 **skip** 来忽略它们。

**name** 可以用来指定与函数跨度 **span** 相关的信息--如果省略，则默认为函数名称。 我们还可以使用字段 **fields** 指令来丰富 **span** 的上下文。它利用了我们已经见过的 **info_span！** 宏的相同语法 。
结果是相当不错的：所有的工具化关注点都被执行关注点直观地分开。 - 前者在程序性宏中处理，"装饰 "了函数声明，而函数主体则侧重于实际的业务逻辑。 
需要指出的是，**tracing::instrument** 也要注意使用 **Instrument::instrument** 如果它被应用于一个异步函数。 
让我们在自己的函数中提取 **query**，并使用 **tracing::instrument** 来摆脱 **query_span**。 以及对 **.instrument** 方法的调用

```rust
//! src/routes/subscriptions.rs
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    match insert_subscriber(&pool, &form).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(pool: &PgPool, form: &FormData) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
        // Using the `?` operator to return early
        // if the function failed, returning a sqlx::Error
        // We will talk about error handling in depth later!
    })?;
    Ok(())
}
```

现在错误事件确实属于查询范围，我们有了更好的分离。 

- **insert_subscriber** 负责数据库逻辑，它对周围的Web框架没有任何交互--也就是说，我们没有把 **web::Form** 或 **web::Data** 包装器作为输入类型传递。 
- **subscribe** 通过调用所需的例程来协调要做的工作，并根据 **HTTP** 协议的规则和惯例将其结果转化为适当的响应。 

我必须承认我对 **tracing::instrument** 无限的热爱：它大大降低了对你的代码进行检测的工作量。 它把你推到 **pit of success**：简单而且正确。

### 5.13  Protect Your Secrets - secrecy

实际上 **#[tracing::instrument]** 有一个元素我并不喜欢：它自动将传递给函数的所有参数附加到 **span** 的上下文中--你必须选择退出 **opt-out** 记录函数输入（通过 **skip**）而不是选择进入 **opt-in**。
你不希望在你的日志中出现秘密（如密码）或个人身份信息（如终端用户的账单地址）。 
**Opt-out** 是一个危险的默认值--每当你使用 **#[tracing::instrument]** 向一个函数添加一个新的输入时， 你都需要问自己：记录这个是否安全？ 我应该跳过它吗？ 
在未来有人可能会忘记 - 你现在有一个安全事件要处理。 你可以通过引入一个明确 **explicitly** 标记哪些字段被认为是敏感字段的-- 使用 **secrecy::Secret** 来防止这种情况发生。

```toml
#! Cargo.toml
# [...]
[dependencies]
secrecy = { version = "0.8", features = ["serde"] }
# [...]
```

让我们来看看它的定义。  

```rust
/// Wrapper type for values that contains secrets, which attempts to limit
/// accidental exposure and ensure secrets are wiped from memory when dropped.
/// (e.g. passwords, cryptographic keys, access tokens or other credentials)
///
/// Access to the secret inner value occurs through the [...]
/// `expose_secret()` method [...]
pub struct Secret<S>
    where
    	S: Zeroize,
{
    /// Inner secret value
    inner_secret: S,
}
```

由归零特性提供的记忆清除是一个不错的选择。Memory wiping, provided by the **Zeroize trait**, is a nice-to-have.

我们正在寻找的关键属性是 **Secret ** 的屏蔽 **Debug** 实现：`println!("{:?}", my_secret_string) ` 输出 `Secret([REDACTED String]) ` 而不是实际的秘密值 。这正是我们需要的，以防止通过 `#[tracing::instrument]` 或其他日志语句意外泄漏敏感材料。 
明确的封装类型还有一个额外的好处：它可以作为新的开发人员被引入代码库的文件。在你的领域里/根据相关规定，它可以确定什么是敏感的。
现在，我们唯一需要担心的秘密是数据库密码。让我们把它修改一下。 

```rust
//! src/configuration.rs
use secrecy::Secret;
// [..]

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
	// [...]
	pub password: Secret<String>,
}
```

**Secret** 不干涉反序列化 - **Secret** 通过委托给包装类型的反序列化逻辑来实现 **serde::Deserialize**（如果你像我们一样启用 **serde** 特性标志）。此时编译器会报错。

```bash
error[E0277]: `Secret<std::string::String>` doesn't implement `std::fmt::Display`
--> src/configuration.rs:29:28
|
| 			self.username, self.password, self.host, self.port
| 							^^^^^^^^^^^^^
| `Secret<std::string::String>` cannot be formatted with the default formatter
```

这是一个特性，而不是一个错误-- **secret::Secret** 没有实现 **Display**，因此我们需要明确地允许暴露 **wrapped secret**。编译器错误是一个很好的提示，让我们注意到整个数据库连接字符串也应该被标记为 **Secret**，因为它嵌入了数据库密码。

```rust
//! src/configuration.rs
use secrecy::ExposeSecret;
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        ))
    }

    pub fn connection_string_without_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        ))
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("configuration"))
        .build()?;

    settings.try_deserialize()
}
```

```rust
//! src/main.rs
use secrecy::ExposeSecret;
// [...]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // [...]
    let connection_pool =
        PgPool::connect(&configuration.database.connection_string().expose_secret())
            .await
            .expect("Failed to connect to Postgres.");
    // [...]
}	
```

```rust
//! tests/health_check.rs
use secrecy::ExposeSecret;
// [...]

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection =
        PgConnection::connect(&config.connection_string_without_db().expose_secret())
            .await
            .expect("Failed to connect to Postgres");
    // [...]
    let connection_pool = PgPool::connect(&config.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");
    // [...]
}
```

这就是目前的情况--今后我们会将更多敏感信息放入 **Secret**。

### 5.14  Request Id

我们还有最后一项工作要做：确保某个特定请求的所有日志，特别是有返回状态码的记录，都有一个 **request_id** 属性来充实。怎么做？ 
如果我们的目标是避免使用 **actix_web::Logger**，最简单的解决方案是添加另一个中间件。 **RequestIdMiddleware**，来负责。 

- 产生一个独特的请求标识符。 
- 创建一个新的跨度，并将请求标识符作为上下文。 
- 将中间件链的其余部分包裹在新创建的跨度中。 

不过我们会留下很多东西：**actix_web::Logger** 并不能让我们以与其他日志相同的结构化 **JSON** 格式访问其丰富的信息（状态代码、处理时间、调用者IP等）-- 我们必须从其消息字符串中解析出所有这些信息。在这种情况下，我们最好是引入一个可追踪的解决方案。 让我们把 **tracing-actix-web** 作为我们的一个依赖项。

```toml
#! Cargo.toml
# [...]
[dependencies]
tracing-actix-web = "0.5"
# [...]
```

它是作为 **actix-web** 的 **Logger** 的直接替代品而设计的，只是基于 **tracing** 而不是 **log**。

```rust
//! src/startup.rs
use actix_web::dev::Server;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::routes::{health_check, subscribe};

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            // Instead of `Logger::default`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
```

如果你启动应用程序并发出一个请求，你应该在所有的日志上看到一个 **request_id**，以及 **request_path** 和其他一些有用的信息。 
我们几乎已经完成了--有一个悬而未决的问题需要我们去处理。 让我们仔细看看 **POST /subscriptions** 请求所发出的日志记录。 

```shell
{
    "msg": "[REQUEST - START]",
    "request_id": "21fec996-ace2-4000-b301-263e319a04c5",
    ...
}
{
    "msg": "[ADDING A NEW SUBSCRIBER - START]",
    "request_id":"aaccef45-5a13-4693-9a69-5",
    ...
}
```

对于同一个请求，我们有两个不同的 **request_id**! 这个错误可以追溯到我们的订阅函数上的 `#[tracing::instrument]` 注释。 

```rust
//! src/routes/subscriptions.rs
// [...]

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name= %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    // [...]
}
// [...]

```

我们仍然在函数级生成一个 `request_id`，它覆盖了来自`TracingLogger` 的 **request_id**。

让我们来修复这个问题

```rust
//! src/routes/subscriptions.rs
// [...]
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name= %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
	// [...]
}
// [...]
```

现在一切都好了--我们的应用程序的每个端点都有一个一致的 **request_id**。

### 5.15    Leveraging The tracing Ecosystem

我们涵盖了 **tracing** 所能提供的很多东西--它大大改善了我们正在收集的遥测数据的质量，以及我们的 **instrumentation** 代码的清晰度。 
同时，当涉及到用户层时，我们几乎没有触及到整个 **tracing** 生态系统的丰富性。 
仅仅再提几个现成的例子。 

**tracing-actix-web** 是与 **OpenTelemetry**兼容的。 如果你插入 [**tracing-opentelemetry**](https://docs.rs/tracing-opentelemetry)，你可以将 **span** 运送到与[**OpenTelemetry**](https://opentelemetry.io/) 兼容的服务（例如 [`Jaeger`](https://www.jaegertracing.io/) 或 [`Honeycomb.io`](https://honeycomb.io/)）进行进一步分析。 

[`tracing-error`](https://docs.rs/tracing-error) 用 [`SpanTrace`](https://docs.rs/tracing-error/latest/tracing_error/struct.SpanTrace.html) 丰富了我们的错误类型，以方便故障排除。

毫不夸张地说，**tracing** 是Rust生态系统中的一个基础板块。虽然日志是最小的共同点，但 **tracing** 现在已经被确立为整个诊断和基础设施生态系统的重要组成部分了。 

## 6. 总结

我们从一个完全很小的 **actix-web** 应用程序开始，最终获得了高质量的远程测试数据。现在是时候把这个 **newsletter API** 投入使用了! 在下一章，我们将为我们的Rust项目建立一个基本的 **deployment pipeline**。

