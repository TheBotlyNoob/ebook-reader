use dioxus::prelude::*;

mod book;
use book::Book;

pub fn app(cx: Scope) -> Element {
    let book = use_ref(cx, || None);

    let latebook = LateBook(book);

    cx.render(rsx! {
        style {
            include_str!("axist.min.css")
        }
        Fragment {
            if book.read().is_some() {
                book::book(cx, &latebook)
            } else {
                book::open_book(cx, book)
            }
        }
    })
}

/// An [`Option`] that's guaranteed to be [`Some`], connected to a [`UseRef`]
pub struct LateBook<'cx>(&'cx UseRef<Option<Book>>);
impl<'cx> LateBook<'cx> {
    fn with<O>(&self, f: impl FnOnce(&Book) -> O) -> O {
        f(self.0.read().as_ref().expect("LateBook was None"))
    }
    fn with_mut<O>(&self, f: impl FnOnce(&mut Book) -> O) -> O {
        f(self.0.write().as_mut().expect("LateBook was None"))
    }
    fn with_mut_silent<O>(&self, f: impl FnOnce(&mut Book) -> O) -> O {
        f(self.0.write_silent().as_mut().expect("LateBook was None"))
    }
}
