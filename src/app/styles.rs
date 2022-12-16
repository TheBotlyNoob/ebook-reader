use std::fmt::Arguments;

use std::format_args as f;
type Ret = Arguments<'static>;

macro_rules! st {
    ($name:ident, $body:expr) => {
        pub fn $name() -> Ret {
            f!($body)
        }
    };
}

st!(center, "text-align: center;");
