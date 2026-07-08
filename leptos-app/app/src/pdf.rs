#[cfg(feature = "ssr")]
pub mod compiler {
    use std::sync::OnceLock;

    use typst::diag::{FileError, FileResult, SourceResult};
    use typst::foundations::{Bytes, Datetime, Duration};
    use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
    use typst::text::{Font, FontBook};
    use typst::utils::LazyHash;
    use typst::{Feature, Features, Library, LibraryExt, World};
    use typst_html::HtmlDocument;
    use typst_layout::PagedDocument;

    use chrono::Datelike;

    /// Fonts are identical for every render and scanning `/usr/share/fonts`
    /// takes long enough to notice, so load them once and share across compiles.
    struct Fonts {
        book: LazyHash<FontBook>,
        fonts: Vec<Font>,
    }

    fn fonts() -> &'static Fonts {
        static FONTS: OnceLock<Fonts> = OnceLock::new();
        FONTS.get_or_init(|| {
            let mut fonts = Vec::new();

            // Typst's own bundle first, so a container with no system fonts still
            // renders real glyphs instead of tofu.
            for data in typst_assets::fonts() {
                fonts.extend(Font::iter(Bytes::new(data)));
            }

            for entry in walkdir::WalkDir::new("/usr/share/fonts")
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path
                    .extension()
                    .map_or(false, |ext| ext == "ttf" || ext == "otf")
                {
                    if let Ok(data) = std::fs::read(path) {
                        fonts.extend(Font::iter(Bytes::new(data)));
                    }
                }
            }

            Fonts {
                book: LazyHash::new(FontBook::from_fonts(&fonts)),
                fonts,
            }
        })
    }

    pub struct KlubuWorld {
        library: LazyHash<Library>,
        main: FileId,
        source: Source,
    }

    impl KlubuWorld {
        pub fn new(text: String) -> Self {
            // `Feature::Html` gates the HtmlDocument output target. It is inert for
            // the PDF path, and required for the HTML one.
            let library = Library::builder()
                .with_features(Features::from_iter([Feature::Html]))
                .build();

            let vpath = VirtualPath::new("main.typ").expect("static path is valid");
            let main = FileId::new(RootedPath::new(VirtualRoot::Project, vpath));
            let source = Source::new(main, text);

            Self {
                library: LazyHash::new(library),
                main,
                source,
            }
        }
    }

    impl World for KlubuWorld {
        fn library(&self) -> &LazyHash<Library> {
            &self.library
        }

        fn book(&self) -> &LazyHash<FontBook> {
            &fonts().book
        }

        fn main(&self) -> FileId {
            self.main
        }

        fn source(&self, id: FileId) -> FileResult<Source> {
            if id == self.main {
                Ok(self.source.clone())
            } else {
                Err(not_found(id))
            }
        }

        /// Templates are self-contained: everything they need is inlined into the
        /// markup before compilation, so no template may read from disk.
        fn file(&self, id: FileId) -> FileResult<Bytes> {
            Err(not_found(id))
        }

        fn font(&self, index: usize) -> Option<Font> {
            fonts().fonts.get(index).cloned()
        }

        fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
            let now = chrono::Local::now();
            Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8)
        }
    }

    fn not_found(id: FileId) -> FileError {
        FileError::NotFound(id.get().vpath().get_with_slash().into())
    }

    fn finish<T>(result: SourceResult<T>) -> Result<T, String> {
        result.map_err(|errs| {
            errs.into_iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })
    }

    /// Compiles Typst markup to PDF bytes.
    pub fn compile_typst(markup: String) -> Result<Vec<u8>, String> {
        let world = KlubuWorld::new(markup);
        let document = finish(typst::compile::<PagedDocument>(&world).output)?;
        finish(typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()))
    }

    /// Compiles Typst markup to a standalone HTML document.
    ///
    /// A template written for paged output will not compile here unchanged:
    /// `#set page(..)` and friends are paged-only. Templates that want both
    /// outputs must guard layout rules with `#if target() == "paged"`.
    pub fn compile_typst_html(markup: String) -> Result<String, String> {
        let world = KlubuWorld::new(markup);
        let document = finish(typst::compile::<HtmlDocument>(&world).output)?;
        finish(typst_html::html(&document, &typst_html::HtmlOptions::default()))
    }
}
