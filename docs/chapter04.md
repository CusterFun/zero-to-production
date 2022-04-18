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

### 5.5

### 5.6

### 5.7

### 5.8

### 5.9

### 5.10

### 5.11

### 5.12

### 5.13

### 5.14

### 5.15

## 6. 总结



