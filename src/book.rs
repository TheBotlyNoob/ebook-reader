use crate::LateBook;
use base64::prelude::*;
use dioxus::prelude::*;
use epub::doc::EpubDoc;
use std::io::Cursor;
use tracing::info;

/// Creates a `data:image/` url from a byte slice
pub fn data_url_from_bytes(bytes: &[u8]) -> String {
    let mut data = String::from("data:image/png;base64,");
    data.push_str(&BASE64_STANDARD.encode(bytes));
    data
}

pub fn book<'cx>(cx: Scope<'cx>, book: LateBook<'cx>) -> Element<'cx> {
    let reading = use_state(cx, || false);

    cx.render(rsx! {
        Fragment {
            style {
                "
                display: flex;
                flex-direction: column;
                height: 100%;
                width: 100%;
                "
            },
            if **reading {
                read(cx, book)
            } else {
                info(cx, book, reading)
            }
        }
    })
}

fn read<'cx>(cx: Scope<'cx>, book: LateBook) -> Element<'cx> {
    book.with_mut_silent(|book| {
        book.doc.go_next();
        let (mut current_page, _mime) = book.doc.get_current_str().unwrap();

        info!(stripped = ?current_page.len());

        let mut stripped = String::with_capacity(current_page.len());

        while let Some(start) = current_page.find("<style") {
            let end = current_page[start..].find("</style>").unwrap() + start + 8;
            stripped.push_str(&current_page[..start]);
            stripped.push_str(&current_page[end..]);
            current_page = current_page[end..].to_owned();
        }
        // get all src attributes and replace them with data urls
        while let Some(mut start) = current_page.find("src=\"") {
            start += 5;
            let end = current_page[start..].find('"').unwrap() + start;
            let mut src = current_page[start..end].replace("../", "");

            let data = if src.starts_with("data:") {
                src
            } else {
                if !src.starts_with("OEBPS") {
                    src = format!("OEBPS/{}", src);
                }
                println!("src: {}", src);
                let data = book.doc.get_resource_by_path(&src).unwrap();
                data_url_from_bytes(&data)
            };

            stripped.push_str(&current_page[..start]);
            stripped.push_str(&data);
            current_page = current_page[end..].to_owned();
        }

        cx.render(rsx! {
            article {
                flex: 1,
                overflow: "auto",
                width: "100%",
                height: "100%",

                iframe {
                    srcdoc: "{stripped}",
                    width: "100%",
                    height: "100%",
                    style {
                        "border: none;"
                    }
                }
            }
        })
    })
}

fn info<'cx>(cx: Scope<'cx>, book: LateBook<'cx>, reading: &'cx UseState<bool>) -> Element<'cx> {
    book.with(|book| {
        cx.render(rsx! {
            h1 {
                book.title.clone()
            }
            h2 {
                book.author.clone()
            }
            img {
                src: "{data_url_from_bytes(book.cover.as_deref().unwrap_or_default())}",
            }
            article {
                dangerous_inner_html: "{book.desc}",
            }
            button {
                onclick: |_| {
                    reading.set(true)
                },

                "Read Book"
            }
        })
    })
}

pub fn open_book<'cx>(cx: Scope<'cx>, book: &'cx UseRef<Option<Book>>) -> Element<'cx> {
    cx.render(rsx! {
        button {
            onclick: move |_| {
                let book = book.to_owned();
                cx.spawn(async move {
                    book.set(open().await);
                })
            },

            "Open Book"
        }
    })
}

type Epub = EpubDoc<Cursor<Vec<u8>>>;

#[derive(Clone, Debug)]
pub struct Book {
    doc: Epub,
    title: String,
    author: String,
    desc: String,
    cover: Option<Vec<u8>>,
}

impl Book {
    fn new(mut doc: Epub) -> Self {
        Self {
            title: doc.mdata("title").unwrap_or_default(),
            author: doc.mdata("author").unwrap_or_default(),
            desc: doc.mdata("description").unwrap_or_default(),
            cover: doc.get_cover().map(|(c, _)| c),
            doc,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn open() -> Option<Book> {
    let f = rfd::AsyncFileDialog::new()
        .add_filter("book", &["epub"])
        .pick_file()
        .await?;
    let doc = EpubDoc::from_reader(Cursor::new(f.read().await)).ok()?;
    Some(Book::new(doc))
}

#[cfg(target_arch = "wasm32")] // I know `rfd` supports wasm, but it doesn't really work how I want it to
async fn open() -> Option<Book> {
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
    rx.next().await.unwrap()
}
