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

    use chrono::{Datelike, Timelike};

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
        /// Files `pdf.attach` may read. Empty for every render but ZUGFeRD.
        attachments: std::collections::HashMap<FileId, Bytes>,
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
                attachments: std::collections::HashMap::new(),
            }
        }

        /// Makes `name` readable from the markup, so `pdf.attach(name)` resolves.
        ///
        /// Serving the bytes through the `World` rather than inlining them into
        /// the markup keeps us from having to escape a whole XML document into a
        /// Typst string literal.
        pub fn with_attachment(mut self, name: &str, data: Vec<u8>) -> Self {
            let vpath = VirtualPath::new(name).expect("attachment name is a valid path");
            let id = FileId::new(RootedPath::new(VirtualRoot::Project, vpath));
            self.attachments.insert(id, Bytes::new(data));
            self
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
        /// markup before compilation, so no template may read from disk. The only
        /// readable files are the attachments handed in explicitly.
        fn file(&self, id: FileId) -> FileResult<Bytes> {
            self.attachments
                .get(&id)
                .cloned()
                .ok_or_else(|| not_found(id))
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

    /// Compiles Typst markup to a standalone PDF/A-3b document.
    ///
    /// This is the archival format used for committed documents that do not
    /// carry an embedded e-invoice attachment, such as offers.
    pub fn compile_typst_pdfa(markup: String) -> Result<Vec<u8>, String> {
        use typst_pdf::{PdfOptions, PdfStandard, PdfStandards, Timestamp};

        let world = KlubuWorld::new(markup);
        let document = finish(typst::compile::<PagedDocument>(&world).output)?;
        let standards = PdfStandards::new(&[PdfStandard::A_3b])
            .map_err(|e| format!("PDF/A-3b nicht verfügbar: {}", e.message()))?;
        let now = chrono::Utc::now();
        let timestamp = Datetime::from_ymd_hms(
            now.year(),
            now.month() as u8,
            now.day() as u8,
            now.hour() as u8,
            now.minute() as u8,
            now.second() as u8,
        )
        .map(Timestamp::new_utc);
        let options = PdfOptions {
            standards,
            timestamp,
            ..Default::default()
        };

        finish(typst_pdf::pdf(&document, &options))
    }

    /// Compiles Typst markup to a **ZUGFeRD** PDF: PDF/A-3b with `xml_name`
    /// embedded as an alternative representation of the document.
    ///
    /// Three things are load-bearing and easy to get wrong:
    ///
    /// * The standard must be PDF/A-3. PDF/A-1 and PDF/A-2 forbid attachments,
    ///   and Typst rejects the render rather than dropping the file quietly.
    /// * The relationship must be `alternative` — that is what tells a reader the
    ///   XML *is* the invoice, not a bonus spreadsheet.
    /// * A document date is mandatory when attaching. `set document(date: none)`
    ///   in a template would break this, hence the explicit timestamp.
    pub fn compile_typst_zugferd(
        markup: String,
        xml_name: &str,
        xml: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        use typst_pdf::{PdfOptions, PdfStandard, PdfStandards, Timestamp};

        let attach = format!(
            "#pdf.attach(\"{xml_name}\", relationship: \"alternative\", \
             mime-type: \"application/xml\", description: \"Factur-X/ZUGFeRD invoice data\")\n"
        );
        let world = KlubuWorld::new(format!("{attach}{markup}")).with_attachment(xml_name, xml);

        let document = finish(typst::compile::<PagedDocument>(&world).output)?;

        let standards = PdfStandards::new(&[PdfStandard::A_3b])
            .map_err(|e| format!("PDF/A-3b nicht verfügbar: {}", e.message()))?;

        let now = chrono::Utc::now();
        let timestamp = Datetime::from_ymd_hms(
            now.year(),
            now.month() as u8,
            now.day() as u8,
            now.hour() as u8,
            now.minute() as u8,
            now.second() as u8,
        )
        .map(Timestamp::new_utc);

        let options = PdfOptions {
            standards,
            timestamp,
            ..Default::default()
        };
        finish(typst_pdf::pdf(&document, &options))
    }

    /// Compiles Typst markup to a standalone HTML document.
    ///
    /// A template written for paged output will not compile here unchanged:
    /// `#set page(..)` and friends are paged-only. Templates that want both
    /// outputs must guard layout rules with `#if target() == "paged"`.
    pub fn compile_typst_html(markup: String) -> Result<String, String> {
        let world = KlubuWorld::new(markup);
        let document = finish(typst::compile::<HtmlDocument>(&world).output)?;
        finish(typst_html::html(
            &document,
            &typst_html::HtmlOptions::default(),
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn archival_compiler_produces_pdfa_3b_metadata() {
            let pdf = compile_typst_pdfa("PDF/A offer".into()).expect("PDF/A render");
            let document = lopdf::Document::load_mem(&pdf).expect("parse generated PDF");
            let metadata = document
                .objects
                .values()
                .find_map(|object| {
                    let stream = object.as_stream().ok()?;
                    let is_metadata = stream
                        .dict
                        .get(b"Type")
                        .ok()
                        .and_then(|value| value.as_name().ok())
                        == Some(b"Metadata".as_slice());
                    is_metadata.then(|| {
                        stream
                            .decompressed_content()
                            .unwrap_or_else(|_| stream.content.clone())
                    })
                })
                .expect("read XMP metadata");
            let xmp = String::from_utf8(metadata).expect("XMP metadata is UTF-8");

            assert!(xmp.contains("<pdfaid:part>3</pdfaid:part>"), "{xmp}");
            assert!(
                xmp.contains("<pdfaid:conformance>B</pdfaid:conformance>"),
                "{xmp}"
            );
        }
    }
}
