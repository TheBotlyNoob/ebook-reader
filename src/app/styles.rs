macro_rules! __hack_ignore {
    ($var:expr, $act:expr) => {
        $act
    };
}
pub(crate) use __hack_ignore;

macro_rules! styles {
    ($($style:ident($($arg:expr),*)),*) => {
        {
            styles!( $( styles::$style( $($arg),* ) )* )
        }
    };
    ($($arg:expr)*) => {
        unsafe { std::mem::transmute::<_, std::fmt::Arguments<'static>>(format_args!(concat!($(styles::__hack_ignore!($arg, "{}")),*), $($arg),*)) }
    };
}

pub(crate) use styles;

macro_rules! st {
    ($(#[doc = $doc:expr])? $fn:ident($($arg:ident: $type:ty),*) => $body:expr) => {
        #[allow(dead_code)]
        $(#[doc = $doc])?
        pub fn $fn($($arg: $type),*) -> impl std::fmt::Display {
            $body
        }
    };
}

st!(
/// Center the elements in the container.
container() => "
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
");
