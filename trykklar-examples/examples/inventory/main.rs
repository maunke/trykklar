use anyhow::Result;
use clap::Parser;
use trykklar::inventory::page::{ColorSpacesInventory, ImagesInventory};
use trykklar::pdf::Mm;
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
        let mut colorspaces_inventory = ColorSpacesInventory::new();
        let mut images_inventory = ImagesInventory::new();

        let mut walker = PageWalker::new();
        walker.add_processor(&mut colorspaces_inventory);
        walker.add_processor(&mut images_inventory);
        walker.run(&page)?;

        println!(
            "\n=====================\nColorSpaces on Page {}\nTotal: {}, Errors: \
             {}\n======================\n",
            idx,
            colorspaces_inventory.color_spaces().len(),
            colorspaces_inventory.inderterminate()
        );
        for cs in colorspaces_inventory.color_spaces() {
            println!("- {:?}", cs);
        }
        println!("\nThe following separation names were used:\n");
        for separation_name in colorspaces_inventory.separation_names() {
            println!("- {:?}", separation_name);
        }

        println!(
            "\n=====================\nImages on Page {}\nTotal: {}, Errors: \
             {}\n=====================",
            idx,
            images_inventory.painted_images().len(),
            images_inventory.inderterminate()
        );
        for img in images_inventory.painted_images() {
            let dpi = img.dpi();
            println!("\n- dpi (x, y): ({:.0}, {:.0})", dpi.x(), dpi.y());
            if let Some(bbox) = img.bbox::<Mm>() {
                println!(
                    "  bbox (mm): origin ({:.2}, {:.2}), width x height ({:.2} x {:.2})",
                    bbox.origin.x.get(),
                    bbox.origin.y.get(),
                    bbox.size.width.get(),
                    bbox.size.height.get(),
                );
            }
            println!(
                "  pixel width x height: {:?} x {:?}",
                img.xobject().width()?.get(),
                img.xobject().height()?.get()
            );
            println!("  kind: {:?}", img.xobject().kind()?);
            println!("  bpc: {:?}", img.xobject().bits_per_component()?);
            println!("  stream filter: {:?}", img.xobject().filter()?);
            println!("  has soft mask: {}", img.xobject().soft_mask().is_some());
        }
    }
    Ok(())
}
