use anyhow::Result;
use clap::Parser;
use trykklar::inventory::page::ColorSpacesInventory;
use trykklar::{PageWalker, Pdf};

#[derive(Parser)]
struct Args {
    path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let pdf = Pdf::load(&args.path)?;

    let pages = pdf.pages();

    println!("Processing {} pages on {}", pages.len(), args.path);
    println!("Collecting ColorSpaces painted on...");

    for (idx, page) in pages.into_iter().enumerate() {
        let page = page?;
        let mut walker = PageWalker::new();
        let mut colorspaces_inventory = ColorSpacesInventory::new();
        walker.add_processor(&mut colorspaces_inventory);
        walker.run(&page)?;
        println!(
            "\n=================\nPage {} [Errors {}]\n=================\n",
            idx,
            colorspaces_inventory.inderterminate()
        );
        for cs in colorspaces_inventory.color_spaces() {
            println!("- {:?}", cs);
        }
    }
    Ok(())
}
