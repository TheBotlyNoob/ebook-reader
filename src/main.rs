use dioxus::prelude::*;
use epub::doc::EpubDoc;
use std::{fmt::Debug, io::Cursor};

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

fn main() {
    dioxus::desktop::launch(|cx| {
        let bulma = include_str!("../bulma.min.css");
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
                #main {{
                    display: flex;
                    justify-content: center;
                    align-items: center;
                }}

                {bulma}
                "
            },


            app()
        })
    });
}

fn app(cx: Scope) -> Element {
    let book = use_read(&cx, BOOK);
    cx.render(if let Some(book) = book {
        rsx! {
            h1 {
                style: "color: red;",
                "{book.title}"
            }
        }
    } else {
        let set_book = use_set(&cx, BOOK);

        let onclick = move |_| {
            cx.spawn({
                let set_book = set_book.clone();
                async move {
                    if let Some(f) = rfd::AsyncFileDialog::new()
                        .add_filter("book", &["epub"])
                        .pick_file()
                        .await
                    {
                        if let Ok(doc) = EpubDoc::from_reader(Cursor::new(f.read().await)) {
                            let book = Some(Book::new(doc));
                            println!("{book:#?}");
                            set_book(book);
                        }
                    };
                }
            })
        };

        rsx! {
            button {
                onclick: onclick,
                "click here to open a book"
            }
        }
    })
}
