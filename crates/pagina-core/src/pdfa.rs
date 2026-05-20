/// PDF/A-1b conformance support.
///
/// PDF/A-1b is the minimum conformance level for long-term archival.
/// Requirements:
/// - XMP metadata stream
/// - ICC output intent (sRGB)
/// - All fonts embedded
/// - No encryption
/// - Document metadata (title, author)


/// Options for PDF/A output.
#[derive(Debug, Clone, Default)]
pub struct PdfAOptions {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub conformance: PdfAConformance,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum PdfAConformance {
    #[default]
    PdfA1b,
    PdfA2b,
    PdfA3b,
}

impl PdfAConformance {
    fn part(&self) -> u8 {
        match self {
            PdfAConformance::PdfA1b => 1,
            PdfAConformance::PdfA2b => 2,
            PdfAConformance::PdfA3b => 3,
        }
    }

    fn conformance_level(&self) -> &str {
        "B" // All our levels are "B" (basic)
    }
}

/// Generate XMP metadata for PDF/A conformance.
pub fn generate_xmp_metadata(opts: &PdfAOptions) -> Vec<u8> {
    let now = chrono_now();
    let part = opts.conformance.part();
    let level = opts.conformance.conformance_level();

    let xmp = format!(
        r#"<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>
<x:xmpmeta xmlns:x='adobe:ns:meta/'>
  <rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
    <rdf:Description rdf:about=''
      xmlns:dc='http://purl.org/dc/elements/1.1/'
      xmlns:xmp='http://ns.adobe.com/xap/1.0/'
      xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'
      xmlns:pdf='http://ns.adobe.com/pdf/1.3/'>
      <dc:title>
        <rdf:Alt>
          <rdf:li xml:lang='x-default'>{title}</rdf:li>
        </rdf:Alt>
      </dc:title>
      <dc:creator>
        <rdf:Seq>
          <rdf:li>{author}</rdf:li>
        </rdf:Seq>
      </dc:creator>
      <dc:description>
        <rdf:Alt>
          <rdf:li xml:lang='x-default'>{subject}</rdf:li>
        </rdf:Alt>
      </dc:description>
      <xmp:CreatorTool>Pagina</xmp:CreatorTool>
      <xmp:CreateDate>{now}</xmp:CreateDate>
      <xmp:ModifyDate>{now}</xmp:ModifyDate>
      <pdf:Producer>Pagina (Rust)</pdf:Producer>
      <pdfaid:part>{part}</pdfaid:part>
      <pdfaid:conformance>{level}</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end='w'?>"#,
        title = xml_escape(&opts.title),
        author = xml_escape(&opts.author),
        subject = xml_escape(&opts.subject),
    );

    xmp.into_bytes()
}

/// Post-process PDF bytes to inject PDF/A XMP metadata.
pub fn make_pdfa(pdf_bytes: &[u8], opts: &PdfAOptions) -> Vec<u8> {
    let Ok(mut doc) = lopdf::Document::load_mem(pdf_bytes) else {
        return pdf_bytes.to_vec();
    };

    // Add XMP metadata stream
    let xmp_data = generate_xmp_metadata(opts);
    let xmp_dict = lopdf::Dictionary::from_iter(vec![
        ("Type", lopdf::Object::Name(b"Metadata".to_vec())),
        ("Subtype", lopdf::Object::Name(b"XML".to_vec())),
        ("Length", lopdf::Object::Integer(xmp_data.len() as i64)),
    ]);
    let xmp_stream = lopdf::Stream::new(xmp_dict, xmp_data);
    let xmp_ref = doc.add_object(lopdf::Object::Stream(xmp_stream));

    // Set metadata on catalog
    let output_intent_dict = lopdf::Dictionary::from_iter(vec![
        ("Type", lopdf::Object::Name(b"OutputIntent".to_vec())),
        ("S", lopdf::Object::Name(b"GTS_PDFA1".to_vec())),
        ("OutputConditionIdentifier", lopdf::Object::String(
            b"sRGB IEC61966-2.1".to_vec(), lopdf::StringFormat::Literal,
        )),
        ("RegistryName", lopdf::Object::String(
            b"http://www.color.org".to_vec(), lopdf::StringFormat::Literal,
        )),
    ]);
    let intent_ref = doc.add_object(lopdf::Object::Dictionary(output_intent_dict));

    if let Ok(catalog) = doc.catalog_mut() {
        catalog.set("Metadata", xmp_ref);
        catalog.set("OutputIntents", lopdf::Object::Array(vec![
            lopdf::Object::Reference(intent_ref),
        ]));
    }

    let mut output = Vec::new();
    let _ = doc.save_to(&mut output);
    output
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without external crate
    // Format: 2024-01-15T10:30:00Z
    "2026-01-01T00:00:00Z".to_string() // Placeholder; real impl would use std::time
}
