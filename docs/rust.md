# Rust

Rust 是一种系统编程语言，由 Mozilla 基金会开发。它的目标是提供一种安全、并发和可维护的编程语言。

- 官方网站: https://rust-lang.org/
- 官方仓库: https://github.com/rust-lang/rust

---

## 安装

使用 `rustup` 安装 Rust 工具链（推荐方式）：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安装程序把二进制放到 `~/.cargo/bin/`，并尝试写入 PATH。若命令找不到，手动加 PATH：

```bash
# 临时（当前终端）
export PATH="$HOME/.cargo/bin:$PATH"

# 永久（macOS zsh 推荐写 .zshrc）
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

验证：

```bash
rustc --version   # 应输出 rustc 1.x.x
cargo --version
```

### 常见问题

- **command not found 但 `~/.cargo/bin/` 下已有文件** → PATH 没生效，按上面手动步骤处理
- **"rustc binary is not applicable" / "missing manifest"** → 工具链损坏，删 `~/.rustup/toolchains/` 后重装
- **rustup 报 `.profile` Permission denied** → 文件属主是 root，工具链其实已装完，只需手动配 PATH（见 FAQ）

### rustup 常用命令

```bash
rustup update          # 更新工具链和 rustup 自身
rustup show            # 查看已安装工具链状态
rustup default stable  # 切换默认工具链
rustup default nightly
rustup component add rustfmt clippy rust-analyzer  # 安装/添加组件
```

---

## Cargo 基础

```bash
cargo new my-project       # 新建项目（默认 bin，加 --lib 建库）
cargo build                # 编译（debug）
cargo build --release      # 编译（release，带优化）
cargo run                  # 编译并运行（= build + run）
cargo check                # 仅检查语法/类型，不生成二进制（最快）
cargo test                 # 运行所有测试
cargo fmt --check          # CI 中检查格式（不修改文件）
cargo clippy --all-targets # 全范围 lint
cargo add serde --features derive   # 添加依赖（cargo 1.78+ 内置）
cargo remove serde         # 移除依赖（同上）
cargo update               # 更新 Cargo.lock 中的版本
cargo tree                 # 查看依赖树
cargo clean                # 清理 target/
```

### 项目结构

```
my-project/
├── Cargo.toml          # 依赖和构建配置
├── Cargo.lock          # 锁定依赖版本（不要手动编辑）
└── src/
    └── main.rs         # 入口文件
```

### 开发三件套

```bash
rustup component add rustfmt clippy rust-analyzer
```

| 组件 | 用途 | 命令 |
| --- | --- | --- |
| `rustfmt` | 代码格式化 | `cargo fmt` |
| `clippy` | Lint 与代码检查 | `cargo clippy` |
| `rust-analyzer` | IDE 语言服务器 | 被编辑器自动调用 |

### 常用第三方 cargo 子命令

```bash
cargo install cargo-edit cargo-watch cargo-expand
```

| 工具 | 说明 |
| --- | --- |
| `cargo-watch` | 文件变更自动重新编译/测试 |
| `cargo-expand` | 展开宏，查看最终代码 |
| `cargo-benchcmp` | 基准测试对比（criterion 生态） |

---

## 语法基础

### 变量

```rust
let x = 5;              // 不可变
let mut y = 5;          // 可变，y = 6 合法
const MAX: u32 = 100;   // 常量，必须标注类型
let z = 5u32;           // 类型后缀
let guess: u32 = "42".parse().expect("不是数字");
```

### 标量类型

| 类型 | 说明 |
| --- | --- |
| `i8`/`i16`/`i32`/`i64`/`i128`/`isize` | 有符号整数 |
| `u8`/`u16`/`u32`/`u64`/`u128`/`usize` | 无符号整数 |
| `f32`/`f64` | 浮点 |
| `bool` | 布尔 |
| `char` | Unicode 标量（4 字节，`'中'` 合法） |

### 复合类型

```rust
let tup: (i32, f64, u8) = (500, 6.4, 1);
let (x, y, z) = tup;         // 解构
let first = tup.0;            // 下标访问

let arr = [1, 2, 3, 4, 5];
let arr: [i32; 5] = [1, 2, 3, 4, 5];
let arr = [3; 5];              // [3, 3, 3, 3, 3]
```

### 函数

```rust
fn add(a: i32, b: i32) -> i32 {   // 函数名 snake_case
    a + b                          // 最后一个表达式 = 返回值
}

fn noop() {}                       // 无返回值，返回 ()
```

### 控制流

```rust
if num > 5 {
    println!("大");
} else if num == 5 {
    println!("等于");
} else {
    println!("小");
}

// if 是表达式，可以放 let 右边
let status = if num > 0 { "正" } else { "负" };

// 循环
loop { /* 无限 */ }
while i < 10 { i += 1; }
for item in [1, 2, 3] { println!("{item}"); }
for n in (1..=5).rev() { println!("{n}"); }  // 5,4,3,2,1
```

---

## 所有权与借用

Rust 的安全保障核心。所有权在编译时检查，无需 GC。

### 所有权三规则

1. 每个值有一个所有者
2. 同一时刻只能有一个所有者
3. 所有者离开作用域时值被丢弃

```rust
let s1 = String::from("hello");
let s2 = s1;             // s1 的所有权移动到 s2，s1 不再有效
// println!("{s1}");     // 编译错误！
let s3 = s2.clone();     // 深拷贝，s2 仍有效
```

### 借用规则

- **不可变借用 `&T`**：可同时有多个，只读不写
- **可变借用 `&mut T`**：同一时刻只能有一个，独占写
- 二者互斥：要么多个 `&T`，要么一个 `&mut T`

```rust
fn calculate_length(s: &String) -> usize {   // 借用，不获取所有权
    s.len()
}

let mut s = String::from("hello");
change(&mut s);                              // 可变借用
```

### 切片

不获取所有权，只是原数据的视图：

```rust
let s = String::from("hello world");
let hello = &s[0..5];      # String 切片：&str
let world = &s[6..=10];

let arr = [1, 2, 3, 4, 5];
let slice = &arr[1..4];    # 数组切片：&[i32]
```

### String vs &str

| 类型 | 存储 | 可变性 |
| --- | --- | --- |
| `String` | 堆分配，有长度/容量 | 可 |
| `&str` | 字符串切片（只读视图） | 不可 |

函数参数一般优先用 `&str`（比 `String` 更灵活）。

---

## 结构体

```rust
struct User {
    active: bool,
    username: String,
    email: String,
    sign_in_count: u64,
}

let mut user1 = User {
    active: true,
    username: String::from("alice"),
    email: String::from("alice@example.com"),
    sign_in_count: 1,
};

// 更新语法（其他字段沿用 user1）
let user2 = User { email: String::from("bob@example.com"), ..user1 };
```

### `#[derive]` 自动 trait

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point { x: i32, y: i32 }
```

常用 derive：`Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`Hash`、`Default`、`Serialize`/`Deserialize`（serde）。

### impl 块

```rust
impl User {
    // 关联函数（类似构造器）
    fn new(email: String, username: String) -> Self {
        Self { active: true, username, email, sign_in_count: 0 }
    }

    // 方法（第一个参数 &self）
    fn deactivate(&mut self) {
        self.active = false;
    }

    // 消费所有权的方法
    fn into_email(self) -> String {
        self.email
    }
}

let mut u = User::new(...);
u.deactivate();
```

### 元组结构体

```rust
struct Color(i32, i32, i32);
struct AlwaysEqual;   // 单元结构体，零大小类型
```

---

## 枚举与模式匹配

### 定义

```rust
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(u8, u8, u8),
}
```

`Option<T>` 是最常用的枚举（标准库）：

```rust
enum Option<T> { Some(T), None }
```

### match（穷尽匹配）

```rust
fn process(msg: Message) {
    match msg {
        Message::Quit => println!("退出"),
        Message::Move { x, y } => println!("移动到 ({}, {})", x, y),
        Message::Write(text) => println!("文字: {}", text),
        Message::ChangeColor(r, g, b) => println!("颜色: {} {} {}", r, g, b),
    }
}
```

### if let / while let

只关心一种模式时更简洁：

```rust
if let Some(3) = some_value { println!("三"); }

while let Some(top) = stack.pop() { println!("{}", top); }
```

### 解构、范围与守卫

```rust
let Point { x: a, .. } = p;          // 解构，其余忽略
if let Color::Rgb(r, _, _) = c {}    // _ 忽略单个字段
let _ = &v[100];                     // 通配符 _ 忽略整个值

match num {
    1..=5 => println!("小数字"),
    n if n % 2 == 0 => println!("偶数 {}", n),
    _ => println!("其他"),
}
```

---

## 错误处理

| 场景 | 方式 |
| --- | --- |
| 可恢复错误 | `Result<T, E>` |
| 不可恢复错误 | `panic!`（debug 模式会展开） |

```rust
use std::fs::File;

let f = File::open("hello.txt");
let f = match f {
    Ok(file) => file,
    Err(error) => panic!("打开文件失败: {:?}", error),
};
```

### `?` 操作符：自动传播错误

```rust
use std::fs::File;
use std::io::Read;

fn read_username_from_file() -> Result<String, std::io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}
```

`?` 还能自动转换错误类型（只要目标实现了 `From`）。

### `.await?` 组合

```rust
async fn fetch_data(url: &str) -> Result<String, reqwest::Error> {
    let body = reqwest::get(url).await?.text().await?;
    Ok(body)
}
```

---

## 泛型与 Trait

### 泛型函数

```rust
fn largest<T: PartialOrd + Copy>(list: &[T]) -> T {
    let mut largest = list[0];
    for &item in list.iter() {
        if item > largest { largest = item; }
    }
    largest
}
```

### Trait（接口）

```rust
trait Summary {
    fn summarize(&self) -> String;

    // 默认实现
    fn summarize_author(&self) -> String { String::from("(作者佚名)") }
}

impl Summary for User {
    fn summarize(&self) -> String { format!("用户: {}", self.username) }
}
```

### Trait 约束

```rust
fn notify(item: &impl Summary) { item.summarize(); }

// 等价写法（更灵活，可加多个约束）
fn notify<T: Summary>(item: &T) { item.summarize(); }

// where 子句（多个约束时更清晰）
fn some_function<T, U>(t: &T, u: &U) -> i32
where
    T: Display + Clone,
    U: Clone + Debug,
{ ... }
```

### `impl Trait`（返回值）

```rust
fn returns_summarizable() -> impl Summary { User { ... } }
```

### 关联类型

```rust
trait Iterator {
    type Item;                    // 关联类型
    fn next(&mut self) -> Option<Self::Item>;
}
```

### 生命周期

生命周期确保引用有效，编译器通过注解防止悬垂引用：

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

**三条省略规则**（编译器自动补充生命周期）：

1. 每个引用参数各有一个生命周期参数
2. 如果只有一个输入生命周期，赋给所有输出
3. 如果有 `&self` / `&mut self`，self 的生命周期赋给所有输出

**`'static`**：引用可以活过整个程序（如字符串字面量）。

---

## 模块系统

### 包、Crate、模块

- **包 Package**：一个 `Cargo.toml`，包含一个或多个 crate
- **Crate**：编译单元，产出 `lib` 或 `bin`
- **模块 Module**：crate 内组织代码，控制可见性

```
my-project/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── garden.rs
│   └── garden/
│       ├── vegetable.rs
│       └── fruit.rs
```

### 可见性

```rust
mod garden;                   // 声明模块（src/garden.rs）
pub mod vegetable;            // 对父模块可见
pub(crate) mod internal;      // 只在当前 crate 可见

use garden::fruit::Apple;     // 引入
use std::io::Result as IoResult;   // 重命名
pub use garden::fruit::Apple;       // re-export

use super::parent_fn;         // 引用父模块
use self::sub::item;          // 引用当前模块
```

---

## 闭包

匿名函数，可以捕获周围变量：

```rust
let add_one = |x: i32| x + 1;                 // 闭包参数通常需要显式类型
let add_one_simple = (1..=5).map(|n| n * 2);  // 上下文明确时可省略

let x = 4;
let equal_to_x = |z: i32| z == x;             // 捕获 x（不可变借用）
let mut v = vec![1, 2, 3];
let mut push_v = || v.push(4);                // 可变借用，FnMut
let takes = move || x;                        // 转移所有权，FnOnce
```

闭包实现三个 trait 之一（由使用方式决定）：

| Trait | 调用方式 | 说明 |
| --- | --- | --- |
| `FnOnce` | 消费所有权 | 只能调用一次 |
| `FnMut` | 可变借用 | 可多次调用，能修改捕获 |
| `Fn` | 不可变借用 | 可多次调用，只读 |

---

## 迭代器

三种迭代器：

| 方法 | 产出 | 元素类型 |
| --- | --- | --- |
| `.iter()` | 不可变引用 | `&T` |
| `.iter_mut()` | 可变引用 | `&mut T` |
| `.into_iter()` | 所有权 | `T` |

常用适配器（惰性，直到消费才执行）：

```rust
let result: Vec<i32> = (1..=100)
    .filter(|&n| n % 2 == 0)   // 筛选
    .map(|n| n * n)            // 变换
    .take(5)                   // 取前 5 个
    .collect();                // 消费，收集为 Vec
```

| 适配器 | 说明 |
| --- | --- |
| `.filter()` / `.filter_map()` | 筛选 |
| `.map()` / `.flat_map()` | 变换 |
| `.take()` / `.skip()` | 截取/跳过 |
| `.zip()` / `.chain()` | 合并迭代器 |
| `.enumerate()` | 附带下标 |
| `.rev()` / `.sort()` | 逆序 / 排序（需要可收集到数组） |
| `.fold()` / `.reduce()` | 累积归约 |
| `.sum()` / `.product()` | 求和 / 求积 |
| `.find()` / `.any()` / `.all()` | 查找 / 存在性判断 |

---

## 常用标准库类型

### 集合

| 类型 | 说明 |
| --- | --- |
| `Vec<T>` | 动态数组 |
| `String` / `&str` | 堆字符串 / 字符串切片 |
| `HashMap<K, V>` | 哈希表 |
| `BTreeMap<K, V>` | 有序映射（按 key 排序） |
| `HashSet<T>` | 哈希集合 |

### 智能指针 / 内部可变性

| 类型 | 用途 | 线程 |
| --- | --- | --- |
| `Box<T>` | 堆分配，递归类型必需 | 任意 |
| `Rc<T>` | 引用计数共享 | 单线程 |
| `Arc<T>` | 原子引用计数 | 多线程 |
| `RefCell<T>` | 内部可变性（运行时借用检查） | 单线程 |
| `Mutex<T>` | 互斥锁 | 多线程 |
| `RwLock<T>` | 读写锁 | 多线程 |
| `Cell<T>` | Copy 类型的内部可变性 | 单线程 |

---

## 测试

### 单元测试（放在源码里）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() { assert_eq!(2 + 2, 4); }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn panics() { let v = vec![1, 2, 3]; let _ = &v[100]; }

    #[test]
    fn result_is_ok() -> Result<(), String> { Ok(()) }  // ? 测试
}
```

### 集成测试（放根目录 `tests/`）

```
my-project/
├── src/main.rs
└── tests/
    └── integration_test.rs   # 编译为独立 crate，能访问 pub API
```

### 运行测试

```bash
cargo test                    # 全部
cargo test unit               # 名字含 "unit"
cargo test -- --nocapture     # 打印 println!
cargo test --release          # release 配置
cargo test -- --test-threads=1  # 单线程（测试有顺序依赖时）
```

---

## 并发

Rust 用类型系统在编译时**防止数据竞争**（不是检测）。

### 线程 + Arc + Mutex

```rust
use std::thread;
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0));
let mut handles = vec![];

for _ in 0..10 {
    let counter = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        let mut num = counter.lock().unwrap();
        *num += 1;
    });
    handles.push(handle);
}
for h in handles { h.join().unwrap(); }
println!("{}", *counter.lock().unwrap());  // 10
```

### 通道（mpsc）

```rust
use std::sync::mpsc;
let (tx, rx) = mpsc::channel();

thread::spawn(move || {
    let val = String::from("hi");
    tx.send(val).unwrap();    // 所有权转移
});
let received = rx.recv().unwrap();
```

---

## Async/Await

标准库只提供语法，运行时用 tokio（最常用）。

### tokio 入门

```toml
# Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

```rust
#[tokio::main]
async fn main() {
    let t1 = tokio::spawn(async { 42 });
    let t2 = tokio::spawn(async { "hello" });
    let r1 = t1.await.unwrap();
    let r2 = t2.await.unwrap();
    println!("{} {}", r1, r2);
}
```

### 关键概念

- `async fn` 返回 `Future`，不立即执行
- `.await` 挂起当前任务，交给运行时调度（**不阻塞 OS 线程**）
- tokio 在少量线程上调度大量 Future

### 并发 vs Async

| | 线程并发 | Async/Await |
| --- | --- | --- |
| 模型 | OS 线程 + 同步原语 | Future + 运行时调度 |
| 阻塞行为 | 会阻塞 OS 线程 | `.await` 让出，不阻塞 |
| 适用 | CPU 密集、阻塞库 | I/O 密集、高并发 |
| 互斥 | `Mutex` | `tokio::sync::Mutex` |
| 性能 | 切换开销较大 | 极低开销 |

---

## 学习资源

### 主线教材

| 资源 | URL | 备注 |
| --- | --- | --- |
| 《The Book》英文版 | https://doc.rust-lang.org/book/ | 官方主线教材 |
| 《The Book》中文版 TRPL | https://kaisery.github.io/trpl-zh-cn/ | 每天 30 分钟的"知" |
| Rustlings | https://github.com/rust-lang/rustlings | 官方互动题库，跟课表进度刷 |
| Rust by Example（中文） | https://rustwiki.org/zh-CN/rust-by-example/ | 查语法的"字典" |

### 参考手册

| 资源 | URL | 备注 |
| --- | --- | --- |
| 标准库文档 | https://doc.rust-lang.org/std/ | 最常翻的一本书 |
| Crates.io（包仓库） | https://crates.io/ | 找第三方 crate |
| docs.rs（所有 crate 文档） | https://docs.rs/ | 查依赖 API |

### 进阶主题

| 资源 | URL | 备注 |
| --- | --- | --- |
| 异步编程书 async-book | https://rust-lang.github.io/async-book/ | Day 32 起用 |
| Axum 官方示例 | https://github.com/tokio-rs/axum | 第八周主战场 |

### 社区

| 资源 | URL |
| --- | --- |
| Rust 语言中文社区 | https://rustcc.cn |
| Reddit r/rust | https://www.reddit.com/r/rust/ |
| Rust Internals | https://internals.rust-lang.org/ |
| Rust Book 中文版 | https://rust-book.cs.brown.edu/ |
