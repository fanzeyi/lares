/// Finds Feed urls on a web page.
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::BufRead;

use crate::error::Result;

/// Parses HTML page to find `<link rel="alternate" />` and extract hrefs.
pub fn find_rel_alternates<B: BufRead>(reader: B) -> Result<Vec<String>> {
    let mut reader = Reader::from_reader(reader);
    reader.check_end_names(false);

    let mut buf = Vec::new();
    let mut result = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name() == b"link" => {
                if e.attributes().fold(false, |acc, attr| {
                    if acc {
                        acc
                    } else if let Ok(attr) = attr {
                        attr.key == b"rel" && attr.value.as_ref() == b"alternate"
                    } else {
                        false
                    }
                }) {
                    if let Some(url) = e
                        .attributes()
                        .filter_map(|attr| {
                            if let Ok(attr) = attr {
                                if attr.key == b"href" {
                                    String::from_utf8(attr.value.into_owned()).ok()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .next()
                    {
                        result.push(url);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err((e, reader.buffer_position()).into());
            }
            _ => (),
        }
    }

    Ok(result)
}
