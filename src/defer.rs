pub struct Defer<F: FnMut()>(pub F);

impl<F: FnMut()> Drop for Defer<F> {
    fn drop(&mut self) {
        self.0()
    }
}

macro_rules! defer {
    ($($t:tt)*) => {
        let _d = $crate::defer::Defer(|| { $($t)* });
    }
}

pub(crate) use defer;
