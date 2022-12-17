macro_rules! st {
    ($fn:ident($($arg:ident: $type:ty),*), $($fmt:tt)*) => {
        #[allow(dead_code)]
        pub fn $fn($($arg: $type,)*) -> std::fmt::Arguments<'static> {
            ::std::format_args!($($fmt)*)
        }
    };
}

st!(
    container(),
    "
    display: flex;
    justify-content: center;
    align-items: center;
    "
);
