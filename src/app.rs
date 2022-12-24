use epub::doc::EpubDoc;
use iced::{
    widget::{button, column, image, row, text},
    Alignment, Application, Command, Element, Length, Settings, Theme,
};
use std::{fmt::Debug, io::Cursor};

macro_rules! col {
    ($($x:expr),* $(,)?) => {
        column!($($x),*)
    };
}

pub fn run() -> iced::Result {
    Counter::run(Settings::default())
}

#[derive(Clone, Default)]
struct Counter {
    book: Option<Book>,
    /// (image data, mime type)
    cover: Option<(Vec<u8>, String)>,
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
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("eBook Reader")
    }

    fn update(&mut self, message: Msg) -> Command<Msg> {
        use Msg::*;
        match message {
            BookOpened(book) => {
                if let Some(mut book) = book {
                    let cover = if let Some(cover) = book.doc.resources.get("coverimagestandard") {
                        Some(cover)
                    } else {
                        book.doc
                            .resources
                            .get(&book.doc.get_cover_id().unwrap_or_default())
                    };

                    if let Some((path, mime)) = cover {
                        let path = path.clone();
                        let mime = mime.clone();

                        let img = book.doc.get_resource_by_path(path).unwrap();

                        self.cover = Some((img, mime));
                    } else {
                        self.cover = None;
                    }

                    self.book = Some(book);
                } else {
                    self.book = None;
                }
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
            col![
                row![
                    text(&book.title).size(50),
                    text(&book.doc.mdata("author").unwrap_or_default())
                ],
                image("hu"),
                text(&book.doc.mdata("description").unwrap_or_default()),
                button("Close book").on_press(Msg::CloseBook)
            ]
        } else {
            col![button("Click here to open a book").on_press(Msg::OpenBook)]
        }
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .spacing(20)
        .padding(20)
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
