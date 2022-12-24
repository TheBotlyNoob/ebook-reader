use epub::doc::EpubDoc;
use iced::{
    alignment::Horizontal,
    widget::{button, column, row, text},
    Alignment, Application, Command, Element, Length, Settings, Theme,
};
use std::{fmt::Debug, io::Cursor};

pub fn run() -> iced::Result {
    Counter::run(Settings::default())
}

struct Counter {
    book: Option<Book>,
}

#[derive(Clone, Debug)]
enum Msg {
    BookOpened(Option<Book>),
    OpenBook,
    CloseBook,
}

impl Application for Counter {
    type Message = Msg;
    type Executor = iced::executor::Default;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Msg>) {
        (Self { book: None }, Command::none())
    }

    fn title(&self) -> String {
        String::from("Counter")
    }

    fn update(&mut self, message: Msg) -> Command<Msg> {
        use Msg::*;
        match message {
            BookOpened(book) => {
                self.book = book;
                Command::none()
            }
            OpenBook => Command::perform(open_book(), Msg::BookOpened),
            CloseBook => {
                self.book = None;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        if let Some(book) = &self.book {
            column![
                row![
                    text(&book.title),
                    text(&book.doc.mdata("author").unwrap_or_default())
                ],
                text(&book.doc.mdata("description").unwrap_or_default()),
                button("Close book").on_press(Msg::CloseBook)
            ]
        } else {
            column![button("Click here to open a book").on_press(Msg::OpenBook)]
        }
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }
}

#[derive(Clone, Debug)]
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
    use iced_web::futures::StreamExt;
    use wasm_bindgen::{closure::Closure, JsCast};
    let doc = web_sys::window()?.document()?;
    let input = doc
        .create_element("input")
        .ok()?
        .dyn_into::<web_sys::HtmlInputElement>()
        .ok()?;
    input.set_accept(".epub");
    input.set_type("file");
    let (tx, mut rx) = iced_web::futures::channel::mpsc::channel(1);
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
