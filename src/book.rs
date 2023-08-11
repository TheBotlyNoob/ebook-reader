use crate::LateBook;
use base64::prelude::*;
use dioxus::prelude::*;
use epub::doc::EpubDoc;
use std::io::Cursor;

/// Creates a `data:image/` url from a byte slice
pub fn data_url_from_bytes(bytes: &[u8]) -> String {
    let mut data = String::from("data:image/png;base64,");
    data.push_str(&BASE64_STANDARD.encode(bytes));
    data
}

pub fn book<'cx>(cx: Scope<'cx>, book: &'cx LateBook) -> Element<'cx> {
    let reading = use_state(cx, || false);

    if **reading {
        read(cx, book)
    } else {
        info(cx, book, reading)
    }
}

fn read<'cx>(cx: Scope<'cx>, book: &'cx LateBook) -> Element<'cx> {
    let stripped = book.with_mut_silent(|book| {
        book.doc.go_next();
        let (current_page, _mime) = book.doc.get_current_str().unwrap();

        let mut stripped = String::with_capacity(current_page.len());

        let mut to_strip = vec![];

        let links = current_page
            .match_indices("<link")
            .zip(current_page.match_indices('>'))
            .map(|((s, _), (e, _))| (s, e))
            .collect::<Vec<_>>();

        tracing::info!(?links);

        let styles = current_page
            .match_indices("<style")
            .zip(current_page.match_indices("</style>"))
            .map(|((s, _), (e, _))| (s, e))
            .collect::<Vec<_>>();

        tracing::info!(?styles);

        to_strip.extend_from_slice(&links);
        to_strip.extend_from_slice(&styles);

        to_strip.sort_by_key(|(s, _)| *s);

        let mut offset = 0;
        for (start, end) in to_strip {
            stripped.push_str(&current_page[offset..start]);
            offset = end;
        }
        stripped.push_str(&current_page[offset..]);

        // get all src attributes and replace them with data urls
        while let Some(mut start) = stripped.find("src=\"") {
            start += 5;

            let end = stripped[start..].find('"').unwrap() + start;
            let mut src = &stripped[start..end];

            while let Some(stripped) = src.strip_prefix("../") {
                src = stripped;
            }

            let mut src = src.to_owned();

            let data = if src.starts_with("data:") {
                src
            } else {
                if !src.starts_with("OEBPS") {
                    src = format!("OEBPS/{}", src);
                }
                tracing::info!("src: {}", src);
                let data = book.doc.get_resource_by_path(&src).unwrap();
                data_url_from_bytes(&data)
            };

            stripped.replace_range(start..end, &data);
        }

        stripped
    });

    cx.render(rsx! {
            article {
                overflow: "auto",
                width: "100%",
                height: "100%",

                dangerous_inner_html: "{stripped}",
            }

            // turn_page(cx, book, Direction::Left)
            // turn_page(cx, book, Direction::Right)
    })
}

fn info<'cx>(cx: Scope<'cx>, book: &'cx LateBook, reading: &'cx UseState<bool>) -> Element<'cx> {
    book.with(|book: &Book| {
        cx.render(rsx! {
            style {
                "
                #main {{
                    text-align: center;
                }}
                "
            }

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
        style {
            "
            #main {{
                text-align: center;
            }}
            "
        }
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

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Left,
    Right,
}

pub fn turn_page<'cx>(cx: Scope<'cx>, book: &'cx LateBook, direction: Direction) -> Element<'cx> {
    cx.render(rsx! {
        style {
            "
            #main {{
                text-align: center;
            }}
            "
        }
        button {
            onclick: move |_| {
                book.with_mut(|book| {
                    match direction {
                        Direction::Left => book.doc.go_prev(),
                        Direction::Right => book.doc.go_next(),
                    }
                });
            },

            format_args!("Turn Page {direction:?}")
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
