use trykklar_pdf::content::write_operator::WriteOperations;
use trykklar_pdf::{self, ContentWalker, Operator, Pdf, Result};

#[test]
fn content_filter_out_text_operators() -> Result<()> {
    let mut pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
    let page = pdf.page(0)?;
    let mut content_walker = ContentWalker::from_page(&page)?;
    let mut write_operations = WriteOperations::new();
    let mut filtered_operations = 0;
    while let Some(step) = content_walker.next_step()? {
        match step.operator() {
            Operator::ShowText(..) => filtered_operations += 1,
            _ => write_operations.push(step.into()),
        }
    }
    assert!(filtered_operations > 0);
    let mut page_mut = pdf.page_mut(0)?;
    page_mut.set_content(write_operations)?;

    let page = pdf.page(0)?;
    let mut content_walker = ContentWalker::from_page(&page)?;
    while let Some(step) = content_walker.next_step()? {
        assert!(!matches!(step.operator(), Operator::ShowText(..)));
    }
    Ok(())
}
