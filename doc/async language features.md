- async language features
- libraries

- OS线程
- 事件驱动编程，回调
- 协程
- actor模型

- rust中的futures是惰性的，并且只有轮询才会进一步执行
- rust中的异步是零成本的
- rust不提供内置运行时
- rust里单线程的和多线程的运行时都可用

- rust中异步的首选替代就是使用OS线程

- OS线程适合少量任务，因为线程会有CPU和内存开销，生成预切换线程都会有代价，闲置的线程也会消耗系统资源

- 异步 极大地降低了 CPU 和内存开销，尤其是再负载大量越过IO 边界的任务，例如服务器和数据库。

- 异步Rust会导致更大的二进制体积，因为异步函数会生成状态机，并且每个可执行文件都会绑定一个异步运行时

- 异步编程并没有优于线程模型

```rust
async function get_two_sites_async() {
    // create two futures
    let future_one = download_async("https://foo.com");
    let future_two = download_async("https://bar.com");

    // run two futures
    join!(future_one, future_two);
}
```

- 需要同时依靠特性和库支持
  - 最基础的traits、类型、函数，例如Future，由标准库提供
  - async/await语法由rust编译器直接支持
  - 很多工具类型、宏、函数由future库提供，
  - 异步代码的执行、IO和任务生成均由“异步运行时”提供支持，例如Tokio和async-std

- rust中不允许在trait中声明异步函数

- 生命周期lifetime
- 固定pinning

- 状态机

- 兼容性考虑
  - 异步的和同步的代码不总是能自由地结合在一起
  - 异步代码之间不总是能自由地结合在一起，一些库依赖特定的运行时环境

- async/ .await是rust的内置语法
- async将代码块转化成 实现了Future trait 的状态机
- 阻塞Future只会 让出（yield）线程控制权，让其他Future继续执行

- async fn 函数返回实现了Future的类型
- block_on()会执行这个future，
- .await不会阻塞当前线程，而是异步等待future完成

```rust
async fn learn_song() -> Song { ... }
async fn sing_song(song: Song) { ... }
async fn dance() { ... }

async fn learn_and_sing() {
    let song = learn_song().await;
    sing_song(song).await;
}

async fn async_main() {
    let f1 = leran_and_sing();
    let f2 = dance();

    futures::join!(f1, f2);
}

fn main() {
    block_on(async_main());
}
```

- Future和异步任务是如何调度的

- Future trait是Rust异步编程中心内容，是一种异步计算，可以产生值

```rust
trait SimpleFuture {
    type Output;
    fn poll(&mut self, wake: fn()) -> Poll<Self::Output>;
}

enum Poll<T> {
    Ready(T),
    Pending,
}
```

- future完成，返回poll::Ready(result)
- future尚未完成，则返回poll::Pending，并安排wake()函数在 Future 准备好进一步执行时调用。
- 当wake()函数调用时，驱动Future的执行器会再次poll使得Future有进展。

- 没有 wake() 函数的话，执行器将无从获知一个 future 是否能有所进展，只能持续轮询（polling） 所有 future。但有了 wake() 函数，执行器就能知道哪些 future 已经准备好轮询了

```rust
pub struct SocketRead<'a> {
    socket: &'a Socket,
}

impl SimpleFuture for SocketRead<'_> {
    type Output = Vec<u8>;

    fn poll(&mut self, wake: fn()) -> Poll<Self::Output> {
        if self.socket.has_data_to_read() {
            Poll::Ready(self.socket.read_buf())
        } else {
            self.socket.set_readable_callbacl(wake);
            Poll::pending
        }
    }
}
```

- Futures的这种模型允许组合多个异步操作而无需立刻分配资源。
- 同时运行多个future或者串行（chaining）future 能够通过零分配（allocation-free）状态机实现

```rust
pub struct Join<FutureA, FutureB> {
    a: Option<FutureA>,
    b: Option<FUtureB>,
}

impl <FutureA, FutureB> SimpleFuture for Join<FutureA, FutureB>
where
    FutureA: SimpleFuture<Output = ()>,
    FutureB: SimpleFuture<Output = ()>,
{
    type Output = ();
    fn poll(&mut self, wake: fn()) -> Poll<Self::Output> {
        if let Some(a) = &mut self.a {
            if let Poll::Ready(()) = a.poll(wake) {
                self.a.take();
            }
        }

        if let Some(b) = &mut self.b {
            if let Poll::Ready(()) = b.poll(wake) {
                self.b.take();
            }
        }

        if self.a.is_none() && self.b.is_none() {
            // Both futures have completed -- we can return successfully
            Poll::Ready(())
        } else {
            // One or both futures returned `Poll::Pending` and still have
            // work to do. They will call `wake()` when progress can be made.
            Poll::Pending
        }
    }

}
```

```rust
pub struct AndThenFut<FutureA, FutureB> {
    first: Option<FutureA>,
    second: FutureB,
}

impl<FutureA, FutureB> SimpleFuture for AndThenFut<FutureA, FutureB>
where
    FutureA: SimpleFuture<Output = ()>,
    FutureB: SimpleFuture<Output = ()>,
{
    type Output = ();
    fn poll(&mut self, wake: fn()) -> Poll<Self::Output> {
        if let Some(first) = &mut self.first {
            match first.poll(wake) {
                // We've completed the first future -- remove it and start on
                // the second!
                Poll::Ready(()) => self.first.take(),
                // We couldn't yet complete the first future.
                Poll::Pending => return Poll::Pending,
            };
        }
        // Now that the first future is done, attempt to complete the second
        self.second.poll(wake)
    }
}
```

- 真正的 future trait
```rust
trait Future {
    type Output;
    fn poll(
        // Note the change from &mut self to Pin<&mut Self>
        self: Pin<&mut Self>,
        // and the change from `wake: fn()` to `cx: &mut Context<'_>`:
        cx: &mut Context<'_>;
    ) -> Poll<Self::output>;
}
```

- Pin: 它能让我们创建不可移动的future类型。 不可移动对象能够储存指向另一字段（field）的指针

- 在 SimpleFuture 里，我们调用函数指针（fn()） 来告诉执行器有future需要轮询。然而，因为 fn() 是仅仅是个函数指针，它不能储存任何信息说明哪个 Future 调用了 wake
- 像Web服务器这样复杂的应用可能有上千不同的连接，带有应该相互隔离来管理的 唤醒器（wakeups）。Context 类型通过提供对 waker 类型的访问来解决这个问题，这些 waker 会唤起持定任务。

- Waker唤醒任务
- 每次future被轮询时， 它是作为一个“任务”的一部分轮询的。
- 任务（Task）是能提交到执行器上 的顶层future
- Waker提供wake()方法来告诉执行器哪个关联任务应该要唤醒。当wake()函数被调用时， 执行器知道Waker关联的任务已经准备好继续了，并且任务的future会被轮询一遍
- clone()

- 通过整合IO感知系统阻塞元件
  - linux，epoll
  - freeBSD, MacOS, kqueue
  - windows, IOCP

- rust 跨平台库mio

```rust
struct IoBlocker {
    /** ... */
}

struct Event {
    id: usize,
    signals: Signals,
}

impl IoBlocker {
    fn new() -> Self { /** ... */ }

    fn add_io_event_interest(
        &self,
        io_object: &IoBlocker,
        event: Event,
    ) { /** ... */}

    fn block(&self) -> Event { /** ... */}
}

struct Socket {
    fn set_readable_callback(&self, waker: Waker) {
        let local_executor = self.local_executor;
        let id = self.id;
        local_executor.event_map.insert(id, waker);
        local_executor.add_io_event_interest(
            &self.socket_file_descriptor,
            Event { id, signals: READABLE },
        );
    }
}

let mut io_blocker = IoBlocker::new();
io_blocker.add_io_event_interset(&socket_1, Event { id: 1, signals: READABLE });
io_blocker.add_io_event_interest(
    &socket_2,
    Event { id: 2, signals: READABLE | WRITABLE },
);
let event = io_blocker.block();

// prints e.g. "Socket 1 is now READABLE" if socket one became readable.
println!("Socket {:?} is now {:?}", event.id, event.signals);
```

- 现在只需要一个执行器线程来接收并分发任何IO事件给特定 Waker，这些 Waker 会唤醒相应 任务，允许执行器在返回来检查更多IO事件之前，驱动更多任务完成