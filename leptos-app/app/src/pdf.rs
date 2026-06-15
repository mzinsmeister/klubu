#[cfg(feature = "ssr")]
pub mod compiler {
    use typst::foundations::{Bytes, Datetime, Smart};
    use typst::syntax::{FileId, Source};
    use typst::text::{Font, FontBook};
    use comemo::Prehashed;
    use typst::{Library, World};
    use typst::diag::FileError;
    use chrono::Datelike;

    pub struct MyWorld {
        library: Prehashed<Library>,
        book: Prehashed<FontBook>,
        fonts: Vec<Font>,
        main: FileId,
        source: Source,
    }

    impl MyWorld {
        pub fn new(text: String) -> Self {
            let library = Prehashed::new(Library::default());
            let mut fonts = Vec::new();
            
            // Search system fonts
            for entry in walkdir::WalkDir::new("/usr/share/fonts")
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "ttf" || ext == "otf") {
                    if let Ok(data) = std::fs::read(path) {
                        let bytes = Bytes::from(data);
                        fonts.extend(Font::iter(bytes));
                    }
                }
            }
            
            let book = Prehashed::new(FontBook::from_fonts(&fonts));
            let main = FileId::new(None, typst::syntax::VirtualPath::new("/main.typ"));
            let source = Source::new(main, text);
            
            Self {
                library,
                book,
                fonts,
                main,
                source,
            }
        }
    }

    impl World for MyWorld {
        fn library(&self) -> &Prehashed<Library> {
            &self.library
        }
        
        fn book(&self) -> &Prehashed<FontBook> {
            &self.book
        }
        
        fn main(&self) -> Source {
            self.source.clone()
        }
        
        fn source(&self, id: FileId) -> Result<Source, FileError> {
            if id == self.main {
                Ok(self.source.clone())
            } else {
                Err(FileError::NotFound(id.vpath().as_rooted_path().to_path_buf()))
            }
        }
        
        fn file(&self, id: FileId) -> Result<Bytes, FileError> {
            Err(FileError::NotFound(id.vpath().as_rooted_path().to_path_buf()))
        }
        
        fn font(&self, index: usize) -> Option<Font> {
            self.fonts.get(index).cloned()
        }
        
        fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
            let now = chrono::Local::now();
            Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8)
        }
    }

    pub fn compile_typst(markup: String) -> Result<Vec<u8>, String> {
        let world = MyWorld::new(markup);
        
        let mut tracer = typst::eval::Tracer::new();
        let doc = typst::compile(&world, &mut tracer)
            .map_err(|errs| {
                let mut s = String::new();
                for err in errs {
                    s.push_str(&format!("{} ", err.message));
                }
                s
            })?;
            
        let pdf_bytes = typst_pdf::pdf(&doc, Smart::Auto, None);
            
        Ok(pdf_bytes)
    }
}
