use dioxus::prelude::*;
use epub::doc::EpubDoc;
use std::{fmt::Debug, io::Cursor};

/// wraps on top of `web_sys::console.log_1`, use it like:
/// ```ignore
/// println!("a is {}", a);
/// ```
#[macro_export]
macro_rules! println {
    ($($t:tt)*) => {{
        web_sys::console::log_1(&format!($($t)*));
    }};
}

static BOOK: Atom<Option<Book>> = |_| None;

struct Book {
    doc: EpubDoc<Cursor<Vec<u8>>>,
    title: String,
}

impl Book {
    pub fn new(doc: EpubDoc<Cursor<Vec<u8>>>) -> Self {
        Self {
            title: doc.mdata("title").unwrap_or_default(),
            doc,
        }
    }
}

impl Debug for Book {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct EpubDoc;
        f.debug_struct("Book").field("doc", &EpubDoc).finish()
    }
}

pub fn root(cx: Scope) -> Element {
    let bulma = include_str!("../bulma.min.css");
    cx.render(rsx! {
        style {
            "
                html, body, #main {{
                    width: 100%;
                    height: 100%;
                }}

                {bulma}
                "
        },


        app()
    })
}

fn app(cx: Scope) -> Element {
    let book = use_read(&cx, BOOK);
    cx.render(if let Some(book) = book {
        rsx! {
            h1 {
                class: "title is-1 has-text-centered",
                "{book.title}"
            }
        }
    } else {
        let set_book = use_set(&cx, BOOK);

        let onclick = move |_| {
            cx.spawn({
                let set_book = set_book.clone();
                async move {
                    set_book(open_book().await);
                }
            })
        };

        rsx! {
            div {
                class: "container",
                button {
                class: "button is-primary",
                    onclick: onclick,
                    "click here to open a book"
                }
            }
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn open_book() -> Option<Book> {
    let f = rfd::AsyncFileDialog::new()
        .add_filter("book", &["epub"])
        .pick_file()
        .await?;
    let doc = EpubDoc::from_reader(Cursor::new(f.read().await)).ok()?;
    let book = Some(Book::new(doc));
    book
}

#[cfg(target_arch = "wasm32")] // I know `rfd` supports wasm, but it doesn't really work how I want it to
async fn open_book() -> Option<Book> {
    use futures::StreamExt;
    use wasm_bindgen::{closure::Closure, JsCast};
    let doc = web_sys::window()?.document()?;
    let input = doc
        .create_element("input")
        .ok()?
        .dyn_into::<web_sys::HtmlInputElement>()
        .ok()?;
    input.set_accept(".epub");
    input.set_type("file");
    let (tx, mut rx) = futures::channel::mpsc::channel(1);
    input
        .add_event_listener_with_callback("change", {
            let input = input.clone();
            Closure::<dyn FnMut()>::new(move || {
                let mut tx = tx.clone();
                println!("file selected");
                let file = input.files().unwrap().get(0).unwrap();
                let reader = web_sys::FileReader::new().unwrap();
                reader.read_as_array_buffer(&file).unwrap();

                let _reader = reader.clone();
                let on_load = Closure::once_into_js(move || {
                    let reader = _reader;
                    let buf = reader.result().unwrap();
                    let buf = js_sys::Uint8Array::new(&buf).to_vec();
                    let Ok(doc) = EpubDoc::from_reader(Cursor::new(buf)) else {
                        tx.try_send(None).unwrap();
                        println!("failed to open book");
                        return;
                    };
                    let book = Some(Book::new(doc));
                    tx.try_send(book).unwrap();
                });
                reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
            })
            .into_js_value()
            .unchecked_ref()
        })
        .unwrap();
    input.click();
    println!("waiting for book...");
    rx.next().await.unwrap()
}
