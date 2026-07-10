-- The contents have been Markdown since the Typst export pipeline replaced
-- raw HTML rendering.  Keep the schema honest about what these fields contain.
ALTER TABLE invoice RENAME COLUMN header_html TO header;
ALTER TABLE invoice RENAME COLUMN footer_html TO footer;
ALTER TABLE offer RENAME COLUMN header_html TO header;
ALTER TABLE offer RENAME COLUMN footer_html TO footer;
