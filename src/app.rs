use poll_promise::Promise;
use rfd::FileHandle;
use std::io::Cursor;
use tracing::debug;

#[derive(Default)]
pub struct App {
    book: Option<Book>,
    file_open_dialog: Option<poll_promise::Promise<Option<FileHandle>>>,
    _get_file: Option<poll_promise::Promise<Vec<u8>>>,
}

pub struct Book {
    doc: epub::doc::EpubDoc<Cursor<Vec<u8>>>,
    title: String,
    page: usize,
}

impl App {
    fn open_book(&mut self, bytes: Vec<u8>, frame: &mut eframe::Frame) {
        debug!("Got {} bytes", bytes.len());
        let book = epub::doc::EpubDoc::from_reader(Cursor::new(bytes)).expect("Valid EPUB file");
        let title = book
            .mdata("title")
            .unwrap_or_else(|| String::from("Untitled"));
        debug!(?title, "Got book");
        #[cfg(not(target_arch = "wasm32"))]
        frame.set_window_title(&title);
        self.book = Some(Book {
            doc: book,
            title,
            page: 0,
        });
    }
    fn open_file_widget(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
    ) {
        ui.label("Drag and drop an EPUB file here");
        if ui.button("Or click here to open a file").clicked() {
            let promise = self.file_open_dialog.get_or_insert_with(|| {
                let (s, p) = Promise::new();
                prokio::spawn_local(async {
                    let f = rfd::AsyncFileDialog::new()
                        .add_filter("book", &["epub"])
                        .pick_file()
                        .await;

                    s.send(f);
                });
                p
            });
            if let Some(p) = &self.file_open_dialog {
                if let Some(Some(file)) = p.ready() {
                    debug!("Got file");
                    self.file_open_dialog = None;
                    let (s, p) = Promise::new();
                    self._get_file = Some(p);
                    prokio::spawn_local(async move {
                        let bytes = file.read().await;
                        s.send(bytes);
                    });
                }
            }
        }

        if !ctx.input().raw.dropped_files.is_empty() {
            let file = &ctx.input().raw.dropped_files[0];
            let bytes = {
                if let Some(bytes) = file.bytes.as_ref() {
                    debug!("Got bytes");
                    bytes.to_vec()
                } else if let Some(ref path) = file.path {
                    debug!("No bytes but got path");
                    std::fs::read(path).unwrap()
                } else {
                    debug!("No path or bytes");
                    return;
                }
            };

            self.open_book(bytes, frame);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(book) = &self.book {
                ui.label(&book.title);
            } else {
                self.open_file_widget(ctx, ui, frame);
            }
        });
    }
}
