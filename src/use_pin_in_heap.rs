use std::marker::PhantomPinned;
use std::pin::Pin;

/// 固定到栈上
/// 固定 !Unpin 类型到堆上，能给我们的数据一个稳定的地址，所以我们知道我们指向的数据不会在被固定之后被移动走。
/// 和在栈上固定相反，我们知道整个对象的生命周期期间数据都会被固定在一处。

#[derive(Debug)]
struct Test {
    a: String,
    b: *const String,
    _marker: PhantomPinned,
}

impl Test {
    fn new(txt: &str) -> Pin<Box<Test>> {
        let t = Test {
            a: txt.to_string(),
            b: std::ptr::null(),
            _marker: PhantomPinned,
        };
        let mut boxed = Box::pin(t);
        let self_ptr: *const String = &boxed.a;
        unsafe { boxed.as_mut().get_unchecked_mut().b = self_ptr };

        boxed
    }

    fn a(self: Pin<&Self>) -> &str {
        &self.get_ref().a
    }

    fn b(self: Pin<&Self>) -> &String {
        unsafe { &*(self.b) }
    }
}

/// 一些函数需要他们协作的future是Unpin的。
/// 为了让这些函数使用不是 Unpin 的 Future 或 Stream，你首先需要这个值固定，
/// 要么用 Box::pin（创建 Pin<Box<T>>）要么使用 pin_utils::pin_mut!（创建 Pin<&mut T>）。
/// Pin<Box<Fut>> 和 Pin<&mut Fut> 都能用作 future，并且都实现了 Unpin。

pub fn test_pin() {
    let test1 = Test::new("test1");
    let test2 = Test::new("test2");

    println!("a: {}, b: {}", test1.as_ref().a(), test1.as_ref().b());
    println!("a: {}, b: {}", test2.as_ref().a(), test2.as_ref().b());
}
