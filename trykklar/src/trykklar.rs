use pdf::Pdf;

/// Trykklar (Danish for 'ready-to-print') as preflight orientated abstraction on top of the
/// low-level [`Pdf`] functionalies.
pub struct Trykklar {
    pdf: Pdf,
}

impl Trykklar {
    /// Initialization via a provided PDF.
    pub fn new(pdf: Pdf) -> Self {
        Self { pdf }
    }

    /// Returns the PDF.
    pub fn pdf(&self) -> &Pdf {
        &self.pdf
    }
}
