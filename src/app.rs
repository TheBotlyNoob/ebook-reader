use dioxus::prelude::*;
use epub::doc::EpubDoc;
use std::{fmt::Debug, io::Cursor};

pub mod styles;
use styles::styles;

/// wraps on top of `web_sys::console.log_1`, use it like:
/// ```ignore
/// println!("a is {}", a);
/// ```
#[macro_export]
macro_rules! println {
    ($($t:tt)*) => {{
        web_sys::console::log_1(&format!($($t)*).into());
    }};
}

static BOOK: AtomRef<Option<Book>> = |_| None;

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
    cx.render(rsx! {
        style {
            "
            * {{
                margin: 0;
                padding: 0;
                box-sizing: border-box;
            }}
            html, body, #main {{
                width: 100%;
                height: 100%;
            }}
            "
        },


        app()
    })
}

fn app(cx: Scope) -> Element {
    let book = use_atom_ref(&cx, BOOK);

    cx.render(if let Some(Book { title, doc }) = &mut *book.write() {
        rsx! {
            h1 {
                "{title}",
            },
            img {
                src: format_args!("{}", {
                    let cover = if let Some(cover) = doc.resources.get("coverimagestandard") {
                        Some(cover)
                    } else {
                        doc.resources.get(&doc.get_cover_id().unwrap_or_default())
                    };

                    if let Some((path, mime)) = cover {
                        let mime = mime.clone();
                        let path = path.clone();
                        let img = doc.get_resource_by_path(path).unwrap();
                        let img = base64::encode(img);

                        format!(
                            "data:{mime};base64,{img}",
                        ) // we need to allocate because... lifetimes?
                    } else {
                        String::new()
                    }
                })
            }
            p {
                [format_args!("{}", doc.mdata("description").unwrap_or_default())]
            }
        }
    } else {
        let onclick = move |_| {
            let book = book.clone();
            cx.spawn(async move {
                *book.write() = open_book().await;
            })
        };

        rsx! {
            div {
                style: styles!(container()),
                button {
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
    Some(Book::new(doc))
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
